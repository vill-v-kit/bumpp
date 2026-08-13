//! 插件底座（ADR-0007）：生态知识（清单识别、版本更新、install 适配、recursive
//! 收集）的单一事实源。静态链按 `matches` 首命中分发（ADR-0007）：
//! JavaScript（JS manifest）→ Cargo（Cargo 清单 + Cargo.lock 定向同步，ADR-0003）→
//! text（文本模板替换，兜底，仅有版本更新能力）。
//!
//! 布局（Rust 一致性限制：同 trait 同类型的 impl 块不可拆分，trait 实现只能与
//! 类型同文件）：根部每文件一个插件类型，方法一行委托到能力子目录的纯函数——
//! - `version/`   版本解析与版本更新判定（只读）
//! - `install/`   生态 install 适配（ADR-0007）
//! - `recursive/` 清单 basename 常量（recursive 收集与默认清单的模式来源）
//!
//! 编排层职责：文件存在性、事件产出、路径归一、install 链走查、写盘段执行。
//! 逐文件更新拆为两段——判定段（插件 `plan`，只读：读取 + 形态判定 + 新版本
//! 计算 + 全部校验）产出 `FilePlan`，写盘段（`execute_plan`）经效应边界消费
//! 计划、零决策。插件附带同步的文件（如 Cargo.toml 带动的 Cargo.lock）紧随
//! 主文件补发 FileUpdated——updated_files 是 git 提交暂存的依据。

use std::error::Error;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::effects::{Effects, RealEffects};
use crate::progress::ProgressEvent;

mod cargo;
pub mod install;
mod javascript;
pub(crate) mod recursive;
mod text;
pub(crate) mod version;

/// 生态：一套工具链及其版本文件与安装机制的集合（ADR-0007）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
  JavaScript,
  Cargo,
}

/// 单次文件更新结果（FileUpdated / FileSkipped 事件来源）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
  Updated,
  Skipped,
  /// 主文件已更新，并附带同步更新了其他文件（绝对路径，已归一化）——
  /// 如 Cargo.toml 带动的 Cargo.lock 定向同步（ADR-0003）
  UpdatedWith(Vec<PathBuf>),
}

/// 逐文件更新计划（判定段 `plan` 的只读产物，写盘段 `execute_plan` 消费）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePlan {
  /// 跳过（up-to-date / 非本生态形态 / 版本字段缺失等——与真实执行同一判定）
  Skipped,
  /// 待写盘文件序列：主文件在前，附带文件（Cargo.lock）随后。全部计算与
  /// 校验已在判定段完成（含 lock 同步预检——失败即报错且清单不改写），
  /// 写盘段零决策
  Updated(Vec<FileWrite>),
}

/// 一次待执行的写盘（路径 + 全文内容 + 错误归口）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWrite {
  /// 绝对路径（归一化后）
  pub path: PathBuf,
  /// 写盘全文（保格式替换后的完整内容）
  pub content: String,
  /// 写失败的错误归口（各通道错误文案的现实形态）
  pub kind: WriteKind,
}

/// 写失败的错误归口：清单/文本通道为 Io + 相对路径文案；Cargo.lock 定向
/// 同步为 Lock + 绝对显示路径文案（ADR-0003 失败即报错）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteKind {
  /// 清单/文本写盘：错误文案携用户清单中的原始相对路径
  Manifest { rel_path: PathBuf },
  /// Cargo.lock 写盘：错误文案携 cwd 锚定的绝对显示路径（ADR-0002）
  CargoLock,
}

#[derive(Debug)]
pub enum FilesError {
  Io {
    message: String,
  },
  /// 清单不可解析（ADR-0003：失败即报错）
  Parse {
    message: String,
  },
  /// Cargo.lock 定向同步失败（ADR-0003：失败即报错）
  Lock {
    message: String,
  },
}

impl fmt::Display for FilesError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io { message } | Self::Parse { message } | Self::Lock { message } => {
        f.write_str(message)
      }
    }
  }
}

impl Error for FilesError {}

#[derive(Debug)]
pub struct InstallError {
  message: String,
}

impl fmt::Display for InstallError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl Error for InstallError {}

/// 版本文件插件：识别文件形态并承载该生态的各项能力（无状态，静态链跨线程共享）
pub(crate) trait VersionFilePlugin: Sync {
  /// 按相对路径（实际比较 basename）判断是否走本通道
  fn matches(&self, rel_path: &Path) -> bool;
  /// 本通道所服务的生态；兜底通道（Text）为 None（不贡献 install 触发，ADR-0007）
  fn ecosystem(&self) -> Option<Ecosystem>;
  /// 本生态的清单 basename 集合（recursive 整树收集的模式来源，ADR-0003 opt-in；
  /// 兜底通道无清单概念，返回空）
  fn manifest_basenames(&self) -> &'static [&'static str];
  /// 从 `path`（绝对路径）提取版本字面量（ADR-0007）；非本生态形态、缺字段、
  /// 读取失败均返回 None——semver 校验由编排层统一承担（上游 semver.valid 门）
  fn read_version(&self, path: &Path) -> Option<String>;
  /// 只读判定段：从 `path`（绝对路径）读取并判定更新计划；`current` / `new`
  /// 为当前与新版本号；`rel_path` 为用户清单中的原始相对路径，仅用于错误
  /// 消息文案；`cwd` 为错误消息中绝对路径（如 Cargo.lock）的显示路径锚点
  /// （ADR-0002）。零写盘——写盘由编排层 `execute_plan` 消费计划执行
  fn plan(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
    cwd: &Path,
  ) -> Result<FilePlan, FilesError>;
  /// 本生态的 install 适配（ADR-0007，spawn 经效应边界）；无适配能力的通道
  /// （Text）返回 None
  fn install(&self, eff: &dyn Effects, cwd: &Path) -> Option<Result<(), InstallError>>;
}

/// 内置有序链（JavaScript → Cargo → Text 兜底）
static PLUGINS: &[&dyn VersionFilePlugin] = &[
  &javascript::JavaScriptPlugin,
  &cargo::CargoPlugin,
  &text::TextPlugin,
];

/// 各生态清单的 recursive 收集模式表（`-r` 整树收集，ADR-0003 opt-in 语义）：
/// 链上各插件声明的 manifest basenames 聚合为 `**/` glob 模式——生态清单知识的
/// 单一事实源，CLI 经 napi 取用，展开与 IGNORED_DIRS 过滤由 normalize_files 承担
pub fn recursive_manifest_globs() -> Vec<String> {
  default_file_patterns(true)
}

/// files 为空时的默认文件清单（ADR-0007）：链上 manifest basenames 的根级并集
/// （glob 展开使不存在的文件自然消失，无需运行时生态探测）；recursive 时升级为
/// `**/` 整树收集模式（与 recursive_manifest_globs 同一张表）
pub fn default_file_patterns(recursive: bool) -> Vec<String> {
  PLUGINS
    .iter()
    .flat_map(|p| p.manifest_basenames())
    .map(|b| {
      if recursive {
        format!("**/{b}")
      } else {
        b.to_string()
      }
    })
    .collect()
}

/// 链分发版本读取（ADR-0007）：首个命中插件提取版本字面量，
/// semver 校验在编排层统一承担（上游 readVersion 的 semver.valid 门）
pub fn dispatch_read_version(rel_path: &Path, abs_path: &Path) -> Option<String> {
  let raw = PLUGINS
    .iter()
    .find(|p| p.matches(rel_path))?
    .read_version(abs_path)?;
  semver::Version::parse(&raw).ok()?;
  Some(raw)
}

/// 按 `rel_path` 分发到首个命中的插件，更新 `abs_path` 指向的文件
/// （`cwd` 为错误消息的显示路径锚点，ADR-0002）
pub fn dispatch_file(
  rel_path: &Path,
  abs_path: &Path,
  current: &str,
  new: &str,
  cwd: &Path,
) -> Result<UpdateOutcome, FilesError> {
  dispatch_file_with(&RealEffects, rel_path, abs_path, current, new, cwd)
}

/// `dispatch_file` 的效应注入形态：判定段（`dispatch_plan`，只读）→
/// 写盘段（`execute_plan`，经效应边界）
pub fn dispatch_file_with(
  eff: &dyn Effects,
  rel_path: &Path,
  abs_path: &Path,
  current: &str,
  new: &str,
  cwd: &Path,
) -> Result<UpdateOutcome, FilesError> {
  let plan = dispatch_plan(rel_path, abs_path, current, new, cwd)?;
  execute_plan(eff, plan, cwd)
}

/// 链分发判定段（只读）：首个命中插件产出更新计划，零写盘
pub fn dispatch_plan(
  rel_path: &Path,
  abs_path: &Path,
  current: &str,
  new: &str,
  cwd: &Path,
) -> Result<FilePlan, FilesError> {
  PLUGINS
    .iter()
    .find(|p| p.matches(rel_path))
    .expect("the TextPlugin fallback always matches")
    .plan(abs_path, rel_path, current, new, cwd)
}

/// 写盘段：消费判定计划，经效应边界逐条写盘（主文件在前、附带文件随后），
/// 零决策——计划内的全部校验已在判定段完成
fn execute_plan(
  eff: &dyn Effects,
  plan: FilePlan,
  cwd: &Path,
) -> Result<UpdateOutcome, FilesError> {
  match plan {
    FilePlan::Skipped => Ok(UpdateOutcome::Skipped),
    FilePlan::Updated(writes) => {
      let mut extra_paths = Vec::new();
      for (index, write) in writes.iter().enumerate() {
        execute_write(eff, write, cwd)?;
        if index > 0 {
          extra_paths.push(write.path.clone());
        }
      }
      Ok(if extra_paths.is_empty() {
        UpdateOutcome::Updated
      } else {
        UpdateOutcome::UpdatedWith(extra_paths)
      })
    }
  }
}

/// 单条写盘执行：错误归口随计划的 WriteKind（文案与现实各通道一致）
fn execute_write(eff: &dyn Effects, write: &FileWrite, cwd: &Path) -> Result<(), FilesError> {
  eff
    .write_file(&write.path, &write.content)
    .map_err(|e| match &write.kind {
      WriteKind::Manifest { rel_path } => FilesError::Io {
        message: format!("failed to write {}: {e}", crate::display::posix(rel_path)),
      },
      WriteKind::CargoLock => FilesError::Lock {
        message: format!(
          "failed to write {}: {e}",
          crate::display::path(cwd, &write.path)
        ),
      },
    })
}

/// `run_installs` 的效应注入形态（spawn 经效应边界；触发判定为纯计算）：
/// 按生态适配触发 install（ADR-0007 的链走查实现）——逐个执行待触发插件的适配
pub fn run_installs_with(
  eff: &dyn Effects,
  cwd: &Path,
  updated_files: &[String],
) -> Result<(), InstallError> {
  for plugin in installs_to_run(updated_files) {
    if let Some(result) = plugin.install(eff, cwd) {
      result?;
    }
  }
  Ok(())
}

/// 更新文件清单 → 待触发生态集合（链序；零生态命中回退 JavaScript，ADR-0007）
pub fn resolve_ecosystems(updated_files: &[String]) -> Vec<Ecosystem> {
  installs_to_run(updated_files)
    .iter()
    .filter_map(|p| p.ecosystem())
    .collect()
}

/// 待触发的插件集合：每个更新文件的首个命中插件按链序去重（Text 命中不触发
/// 任何适配）；零生态命中（仅 Text 通道或无更新文件）回退 JavaScript——与上游
/// `--install`（无条件 JS PM install）行为一致（ADR-0007）
fn installs_to_run(updated_files: &[String]) -> Vec<&'static dyn VersionFilePlugin> {
  let mut indices: Vec<usize> = updated_files
    .iter()
    .filter_map(|f| PLUGINS.iter().position(|p| p.matches(Path::new(f))))
    .collect();
  indices.sort_unstable();
  indices.dedup();
  let mut plugins: Vec<&'static dyn VersionFilePlugin> =
    indices.into_iter().map(|i| PLUGINS[i]).collect();
  if plugins.iter().all(|p| p.ecosystem().is_none()) {
    plugins.push(&javascript::JavaScriptPlugin);
  }
  plugins
}

pub(crate) fn read_text(path: &Path, rel_path: &Path) -> Result<String, FilesError> {
  std::fs::read_to_string(path).map_err(|e| FilesError::Io {
    message: format!("failed to read {}: {e}", crate::display::posix(rel_path)),
  })
}

/// 一次 updateFiles 的结果。
///
/// 以处理顺序的进度事件为唯一事实源（对应上游逐文件 `operation.update` 产生的
/// FileUpdated / FileSkipped）；updated / skipped 路径列表为派生视图（对应上游
/// operation.state 的 updatedFiles / skippedFiles）。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UpdateFilesOutcome {
  events: Vec<(ProgressEvent, String)>,
}

impl UpdateFilesOutcome {
  /// 处理顺序的 (事件, 绝对路径) 序列，供内置打印与观察者闭包消费（ADR-0002）
  pub fn events(&self) -> &[(ProgressEvent, String)] {
    &self.events
  }

  /// 上游 operation.state.updatedFiles
  pub fn updated_files(&self) -> Vec<&str> {
    self
      .events
      .iter()
      .filter(|(e, _)| *e == ProgressEvent::FileUpdated)
      .map(|(_, p)| p.as_str())
      .collect()
  }

  /// 上游 operation.state.skippedFiles
  pub fn skipped_files(&self) -> Vec<&str> {
    self
      .events
      .iter()
      .filter(|(e, _)| *e == ProgressEvent::FileSkipped)
      .map(|(_, p)| p.as_str())
      .collect()
  }
}

/// 上游 `updateFiles`：逐个文件更新版本号，按处理顺序产出 FileUpdated / FileSkipped 事件。
/// 插件附带同步的文件（Cargo.toml 带动的 Cargo.lock，ADR-0003）紧随主文件补发
/// FileUpdated——updated_files 是 git 提交暂存的依据，附带文件必须入列
pub fn update_files(
  files: &[String],
  cwd: &Path,
  current_version: &str,
  new_version: &str,
) -> Result<UpdateFilesOutcome, FilesError> {
  update_files_with(&RealEffects, files, cwd, current_version, new_version)
}

/// `update_files` 的效应注入形态：逐文件「判定段只读 → 写盘段经效应边界」，
/// 顺序与真实执行一致
pub fn update_files_with(
  eff: &dyn Effects,
  files: &[String],
  cwd: &Path,
  current_version: &str,
  new_version: &str,
) -> Result<UpdateFilesOutcome, FilesError> {
  let mut outcome = UpdateFilesOutcome::default();
  for rel_path in files {
    let (modified, extra_paths) = update_file(eff, rel_path, cwd, current_version, new_version)?;
    // 上游事件路径经 path.resolve(cwd, relPath) 归一化（消除 ./ 与 .. 段）
    let abs_path = resolve(cwd, rel_path).to_string_lossy().into_owned();
    let event = if modified {
      ProgressEvent::FileUpdated
    } else {
      ProgressEvent::FileSkipped
    };
    outcome.events.push((event, abs_path));
    for extra in extra_paths {
      outcome.events.push((
        ProgressEvent::FileUpdated,
        extra.to_string_lossy().into_owned(),
      ));
    }
  }
  Ok(outcome)
}

/// 上游 `updateFile`：文件不存在 → skipped；存在则「判定 → 写盘」两段执行。
/// 返回 (主文件是否更新, 插件附带同步的文件路径)
fn update_file(
  eff: &dyn Effects,
  rel_path: &str,
  cwd: &Path,
  current_version: &str,
  new_version: &str,
) -> Result<(bool, Vec<PathBuf>), FilesError> {
  // 归一化后的绝对路径：插件由其向上派生的附带路径（如相邻 Cargo.lock）随之归一
  let path = resolve(cwd, rel_path);
  if !path.exists() {
    return Ok((false, vec![]));
  }
  let outcome = dispatch_file_with(
    eff,
    Path::new(rel_path),
    &path,
    current_version,
    new_version,
    cwd,
  )?;
  Ok(match outcome {
    UpdateOutcome::Updated => (true, vec![]),
    UpdateOutcome::UpdatedWith(extra_paths) => (true, extra_paths),
    UpdateOutcome::Skipped => (false, vec![]),
  })
}

/// Node `path.resolve(cwd, rel)` 的语义化归一：消除 `.` 与 `..` 段（不解符号链接）
pub(crate) fn resolve(cwd: &Path, rel: &str) -> PathBuf {
  let mut out = cwd.to_path_buf();
  for component in Path::new(rel).components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        out.pop();
      }
      Component::RootDir | Component::Prefix(_) => {
        out = PathBuf::from(component.as_os_str());
      }
      Component::Normal(seg) => out.push(seg),
    }
  }
  out
}
