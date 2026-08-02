//! 文件版本更新编排：对齐上游 bumpp v11 `updateFiles`。
//!
//! 各生态的解析与更新由 version-files crate 的插件链承担（ADR-0004）；
//! 本模块只做编排：文件存在性、事件产出、路径归一。

use std::error::Error;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::progress::ProgressEvent;

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

impl From<version_files::UpdateError> for FilesError {
  fn from(e: version_files::UpdateError) -> Self {
    match e {
      version_files::UpdateError::Io { message } => Self::Io { message },
      version_files::UpdateError::Parse { message } => Self::Parse { message },
      version_files::UpdateError::Lock { message } => Self::Lock { message },
    }
  }
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
/// 插件链附带同步的文件（Cargo.toml 带动的 Cargo.lock，ADR-0003）紧随主文件补发
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

/// 上游 `updateFile`：文件不存在 → skipped；存在则经 version-files 插件链分发更新
/// （ADR-0004）。返回 (主文件是否更新, 插件附带同步的文件路径)
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
  let outcome =
    version_files::update_file(Path::new(rel_path), &path, current_version, new_version)?;
  Ok(match outcome {
    version_files::UpdateOutcome::Updated => (true, vec![]),
    version_files::UpdateOutcome::UpdatedWith(extra_paths) => (true, extra_paths),
    version_files::UpdateOutcome::Skipped => (false, vec![]),
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
