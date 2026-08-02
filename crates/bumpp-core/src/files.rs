//! 文件版本更新：编排（对齐上游 bumpp v11 `updateFiles`）+ 各生态插件（ADR-0007）。
//!
//! 生态插件按 `matches` 顺序静态分发，命中即走对应通道（每生态一文件）：
//! `js_manifest`（JS 生态 JSON manifest）→ `cargo_toml`（Cargo 清单 + Cargo.lock
//! 定向同步，ADR-0003）→ `text`（文本模板替换，兜底）。maven / gradle 等未来
//! 生态以同 trait 插件加入本目录（Text 之前）。
//!
//! 编排层职责：文件存在性、事件产出、路径归一；插件附带同步的文件（如
//! Cargo.toml 带动的 Cargo.lock）紧随主文件补发 FileUpdated——updated_files
//! 是 git 提交暂存的依据。

use std::error::Error;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::progress::ProgressEvent;

mod cargo_toml;
mod js_manifest;
mod text;

/// 生态：一套工具链及其版本文件与安装机制的集合（ADR-0008；files/ 与 install/ 共享）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
  Node,
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

/// 版本文件插件：识别文件形态并保格式更新其中的版本号（无状态，静态链跨线程共享）
pub(crate) trait VersionFilePlugin: Sync {
  /// 按相对路径（实际比较 basename）判断是否走本通道
  fn matches(&self, rel_path: &Path) -> bool;
  /// 本通道所服务的生态；兜底通道（Text）为 None（不贡献 install 触发，ADR-0008）
  fn ecosystem(&self) -> Option<Ecosystem>;
  /// 更新 `path`（绝对路径）指向的文件；`current` / `new` 为当前与新版本号；
  /// `rel_path` 为用户清单中的原始相对路径，仅用于错误消息文案
  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
  ) -> Result<UpdateOutcome, FilesError>;
}

/// 内置有序链（JsManifest → CargoToml → Text 兜底）
static PLUGINS: &[&dyn VersionFilePlugin] = &[
  &js_manifest::JsManifestPlugin,
  &cargo_toml::CargoTomlPlugin,
  &text::TextPlugin,
];

/// 按 `rel_path` 分发到首个命中的插件，更新 `abs_path` 指向的文件
pub fn dispatch_file(
  rel_path: &Path,
  abs_path: &Path,
  current: &str,
  new: &str,
) -> Result<UpdateOutcome, FilesError> {
  PLUGINS
    .iter()
    .find(|p| p.matches(rel_path))
    .expect("TextPlugin 兜底必命中")
    .update(abs_path, rel_path, current, new)
}

/// 文件路径所属生态（经链上首个命中插件判定；仅命中兜底通道时为 None）
pub(crate) fn ecosystem_of(rel_path: &Path) -> Option<Ecosystem> {
  PLUGINS
    .iter()
    .find(|p| p.matches(rel_path))
    .and_then(|p| p.ecosystem())
}

pub(crate) fn read_text(path: &Path, rel_path: &Path) -> Result<String, FilesError> {
  std::fs::read_to_string(path).map_err(|e| FilesError::Io {
    message: format!("读取 {} 失败：{e}", rel_path.display()),
  })
}

pub(crate) fn write_text(path: &Path, rel_path: &Path, content: &str) -> Result<(), FilesError> {
  std::fs::write(path, content).map_err(|e| FilesError::Io {
    message: format!("写入 {} 失败：{e}", rel_path.display()),
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
  let mut outcome = UpdateFilesOutcome::default();
  for rel_path in files {
    let (modified, extra_paths) = update_file(rel_path, cwd, current_version, new_version)?;
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

/// 上游 `updateFile`：文件不存在 → skipped；存在则经插件链分发更新。
/// 返回 (主文件是否更新, 插件附带同步的文件路径)
fn update_file(
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
  let outcome = dispatch_file(Path::new(rel_path), &path, current_version, new_version)?;
  Ok(match outcome {
    UpdateOutcome::Updated => (true, vec![]),
    UpdateOutcome::UpdatedWith(extra_paths) => (true, extra_paths),
    UpdateOutcome::Skipped => (false, vec![]),
  })
}

/// Node `path.resolve(cwd, rel)` 的语义化归一：消除 `.` 与 `..` 段（不解符号链接）
fn resolve(cwd: &Path, rel: &str) -> PathBuf {
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
