//! Cargo 生态版本能力（ADR-0003）：toml_edit 保格式更新 `[package].version`，绝不触碰
//! `[dependencies]` 等其他表；并按 crate name 定向同步向上发现的 `Cargo.lock` 的
//! `[[package]]` 条目（同一 toml_edit 机制，不跑 cargo）。
//!
//! 版本形态探测（与 ADR-0003 决策一致）：
//! - `[package].version` 为字面量 → 更新；已是新值 → 跳过
//! - `version.workspace = true`（成员继承）→ 不强写字面量；若本文件即根（含
//!   `[workspace.package].version` 字面量）则更新该字段，否则跳过（根清单作为
//!   显式文件项自行处理）
//! - 仅 `[workspace.package].version` 字面量（虚拟 workspace 根）→ 更新
//! - 皆无 → 跳过；清单不可解析 → 立即报错（显式列入发版清单的文件，失败即报错）
//!
//! Cargo.lock 同步：从清单所在目录向上发现首个 `Cargo.lock`；找不到则仅更新清单
//! （库 crate 可不提交 lock，不视为漂移）。找到则必须同步成功——条目缺失、版本
//! 漂移、lock 解析失败均立即报错（`FilesError::Lock`），且清单不先行改写。

use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Formatted, Item, Table, Value};

use super::super::{read_text, write_text, FilesError, UpdateOutcome};

/// Cargo 清单识别：basename `cargo.toml`（小写比较——与 MANIFEST_BASENAMES
/// 常量的磁盘惯例名无关，识别面保持大小写不敏感）
pub(crate) fn matches(rel_path: &Path) -> bool {
  rel_path
    .file_name()
    .map(|n| {
      n.to_string_lossy()
        .trim()
        .eq_ignore_ascii_case("cargo.toml")
    })
    .unwrap_or(false)
}

/// 版本解析（ADR-0009）：`[package].version` 字面量优先，其次
/// `[workspace.package].version` 字面量；继承形态（`version.workspace = true`）、
/// 读取/解析失败均返回 None；semver 校验由编排层统一承担
pub(crate) fn read_version(path: &Path) -> Option<String> {
  let text = std::fs::read_to_string(path).ok()?;
  let doc = text.parse::<DocumentMut>().ok()?;
  let package_version = doc
    .get("package")
    .and_then(Item::as_table_like)
    .and_then(|p| p.get("version"))
    .and_then(Item::as_str);
  package_version
    .or_else(|| workspace_version_literal(&doc))
    .map(str::to_string)
}

/// 保格式更新清单版本并按需定向同步 Cargo.lock（形态探测见模块头注释）；
/// `cwd` 为错误消息中绝对路径（lock）的显示路径锚点（ADR-0023）
pub(crate) fn update(
  path: &Path,
  rel_path: &Path,
  current: &str,
  new: &str,
  cwd: &Path,
) -> Result<UpdateOutcome, FilesError> {
  let text = read_text(path, rel_path)?;
  // 显式列入发版清单的文件不可解析 = 漂移风险：立即报错（ADR-0003 失败即报错；
  // 与 JsManifest 通道的上游容错 parity 的有意不对称，见 ADR-0003 落地补充）
  let mut doc = text.parse::<DocumentMut>().map_err(|e| FilesError::Parse {
    message: format!("failed to parse {}: {e}", crate::display::posix(rel_path)),
  })?;

  let package = doc.get("package").and_then(Item::as_table_like);
  // `[package].version` 字面量 → 更新并按 crate name 定向同步 lock
  if let Some(v) = package
    .and_then(|p| p.get("version"))
    .and_then(Item::as_str)
  {
    if v == new {
      return Ok(UpdateOutcome::Skipped);
    }
    let name = package
      .and_then(|p| p.get("name"))
      .and_then(Item::as_str)
      .ok_or_else(|| FilesError::Lock {
        message: format!(
          "{} has no [package] name field; cannot sync Cargo.lock by crate name",
          crate::display::posix(rel_path)
        ),
      })?
      .to_string();
    // 先完成全部计算（含 lock 同步校验），失败即报错且清单不改写
    let lock = find_lock(path)
      .map(|lock_path| sync_lock_by_name(&lock_path, &name, current, new, cwd))
      .transpose()?;
    return apply(
      &mut doc,
      VersionLocation::Package,
      lock,
      path,
      rel_path,
      new,
      cwd,
    );
  }

  // version 非字面量（`version.workspace = true` 继承 / 缺失 / 其他形态）：
  // 本文件若含 `[workspace.package].version` 字面量（本文件即根）→ 更新该字段
  // （lock 按成员扫描同步）；否则跳过——真成员的根清单作为显式文件项自行处理
  let Some(v) = workspace_version_literal(&doc) else {
    return Ok(UpdateOutcome::Skipped);
  };
  if v == new {
    return Ok(UpdateOutcome::Skipped);
  }
  let lock = find_lock(path)
    .map(|lock_path| sync_lock_workspace_members(&lock_path, current, new, cwd))
    .transpose()?;
  apply(
    &mut doc,
    VersionLocation::WorkspacePackage,
    lock,
    path,
    rel_path,
    new,
    cwd,
  )
}

/// 从清单所在目录向上发现首个 Cargo.lock（workspace 成员的 lock 位于仓库根）
fn find_lock(manifest_abs: &Path) -> Option<PathBuf> {
  manifest_abs
    .parent()?
    .ancestors()
    .map(|dir| dir.join("Cargo.lock"))
    .find(|p| p.is_file())
}

/// `[workspace.package].version` 的字面量值（本文件即 workspace 根时存在）
fn workspace_version_literal(doc: &DocumentMut) -> Option<&str> {
  doc
    .get("workspace")
    .and_then(Item::as_table_like)
    .and_then(|w| w.get("package"))
    .and_then(Item::as_table_like)
    .and_then(|p| p.get("version"))
    .and_then(Item::as_str)
}

/// 版本字面量改写目标
enum VersionLocation {
  Package,
  WorkspacePackage,
}

/// 应用更新：保格式改写目标表 version → 写清单 → 落 lock
fn apply(
  doc: &mut DocumentMut,
  location: VersionLocation,
  lock: Option<LockSync>,
  path: &Path,
  rel_path: &Path,
  new: &str,
  cwd: &Path,
) -> Result<UpdateOutcome, FilesError> {
  let table = match location {
    VersionLocation::Package => doc.get_mut("package").and_then(Item::as_table_like_mut),
    VersionLocation::WorkspacePackage => doc
      .get_mut("workspace")
      .and_then(Item::as_table_like_mut)
      .and_then(|w| w.get_mut("package"))
      .and_then(Item::as_table_like_mut),
  };
  set_table_version(table, new);
  write_text(path, rel_path, &doc.to_string())?;
  commit_lock(lock, cwd)
}

/// 同步产物：更新后的 lock 文本与其路径（校验全部通过后才允许写盘）
struct LockSync {
  path: PathBuf,
  content: String,
}

/// 按 crate name 定向更新 `[[package]]` 条目：name 匹配且 version == current 的
/// 条目改为新版本；条目缺失或版本漂移均报错（ADR-0003：失败即报错）
fn sync_lock_by_name(
  lock_path: &Path,
  name: &str,
  current: &str,
  new: &str,
  cwd: &Path,
) -> Result<LockSync, FilesError> {
  let mut name_seen = false;
  let (sync, synced) = sweep_lock(lock_path, current, new, cwd, |pkg| {
    let matched = pkg.get("name").and_then(Item::as_str) == Some(name);
    name_seen |= matched;
    matched
  })?;
  if synced == 0 {
    return Err(FilesError::Lock {
      message: match name_seen {
        true => format!(
          "{} has crate \"{name}\" at a version other than the Cargo.toml current version {current} (version drift)",
          crate::display::path(cwd, lock_path)
        ),
        false => format!(
          "{} has no [[package]] entry for crate \"{name}\"",
          crate::display::path(cwd, lock_path)
        ),
      },
    });
  }
  Ok(sync)
}

/// workspace 继承场景：更新所有无 `source`（即 workspace 成员）且
/// version == current 的 `[[package]]` 条目；零匹配视为漂移报错
fn sync_lock_workspace_members(
  lock_path: &Path,
  current: &str,
  new: &str,
  cwd: &Path,
) -> Result<LockSync, FilesError> {
  let (sync, swept) = sweep_lock(lock_path, current, new, cwd, |pkg| {
    pkg.get("source").is_none()
  })?;
  if swept == 0 {
    return Err(FilesError::Lock {
      message: format!(
        "{} has no workspace member entry at version {current} (version drift)",
        crate::display::path(cwd, lock_path)
      ),
    });
  }
  Ok(sync)
}

/// 扫描 lock 的 `[[package]]` 条目：`is_target` 命中且 version == current 的条目
/// 改为新版本；返回更新产物与命中数（零匹配是否报错由调用方裁决）
fn sweep_lock(
  lock_path: &Path,
  current: &str,
  new: &str,
  cwd: &Path,
  mut is_target: impl FnMut(&Table) -> bool,
) -> Result<(LockSync, usize), FilesError> {
  let mut doc = parse_lock(lock_path, cwd)?;
  let mut synced = 0;
  if let Some(packages) = doc
    .get_mut("package")
    .and_then(Item::as_array_of_tables_mut)
  {
    for pkg in packages.iter_mut() {
      if is_target(pkg) && pkg.get("version").and_then(Item::as_str) == Some(current) {
        set_version(pkg.get_mut("version").unwrap(), new);
        synced += 1;
      }
    }
  }
  Ok((
    LockSync {
      path: lock_path.to_path_buf(),
      content: doc.to_string(),
    },
    synced,
  ))
}

/// 清单已写盘后落 lock；返回主文件 + 附带 lock 的更新结果
fn commit_lock(lock: Option<LockSync>, cwd: &Path) -> Result<UpdateOutcome, FilesError> {
  match lock {
    Some(sync) => {
      std::fs::write(&sync.path, &sync.content).map_err(|e| FilesError::Lock {
        message: format!("failed to write {}: {e}", crate::display::path(cwd, &sync.path)),
      })?;
      Ok(UpdateOutcome::UpdatedWith(vec![sync.path]))
    }
    None => Ok(UpdateOutcome::Updated),
  }
}

fn parse_lock(lock_path: &Path, cwd: &Path) -> Result<DocumentMut, FilesError> {
  let text = std::fs::read_to_string(lock_path).map_err(|e| FilesError::Lock {
    message: format!("failed to read {}: {e}", crate::display::path(cwd, lock_path)),
  })?;
  text.parse::<DocumentMut>().map_err(|e| FilesError::Lock {
    message: format!("failed to parse {}: {e}", crate::display::path(cwd, lock_path)),
  })
}

/// 保格式改写字符串值：复制原值的 decor（前后空白与行内注释），仅替换值本体
fn set_version(item: &mut Item, new: &str) {
  if let Some(value) = item.as_value_mut() {
    let decor = value.decor().clone();
    let mut formatted = Formatted::new(new.to_string());
    *formatted.decor_mut() = decor;
    *value = Value::String(formatted);
  }
}

/// 在目标表（`[package]` 或 `[workspace.package]`）上保格式改写 version
fn set_table_version(table: Option<&mut dyn toml_edit::TableLike>, new: &str) {
  if let Some(item) = table.and_then(|t| t.get_mut("version")) {
    set_version(item, new);
  }
}
