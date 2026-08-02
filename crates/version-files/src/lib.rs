//! version-files：版本文件的解析与更新插件底座（ADR-0004）。
//!
//! 内置静态插件链按 `matches` 顺序分发，命中即走对应通道：
//! `JsManifestPlugin`（JS 生态 JSON manifest）→ `CargoTomlPlugin`（Cargo 清单 +
//! Cargo.lock 定向同步，ADR-0003）→ `TextPlugin`（文本模板替换，兜底）。
//! 静态分发，无运行时 registry。

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

mod cargo_toml;
mod js_manifest;
mod text;

/// 单次文件更新结果（对应 bumpp-core 编排层的 FileUpdated / FileSkipped 事件来源）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
  Updated,
  Skipped,
  /// 主文件已更新，并附带同步更新了其他文件（绝对路径，已归一化）——
  /// 如 Cargo.toml 带动的 Cargo.lock 定向同步（ADR-0003）；编排层为附带文件
  /// 补发 FileUpdated 事件，使其进入 updated_files（git 提交暂存依赖该列表）
  UpdatedWith(Vec<PathBuf>),
}

#[derive(Debug)]
pub enum UpdateError {
  Io {
    message: String,
  },
  /// 清单不可解析（显式列入发版清单的文件，失败即报错，ADR-0003）
  Parse {
    message: String,
  },
  /// Cargo.lock 定向同步失败（发版一致性优先，立即报错，ADR-0003）
  Lock {
    message: String,
  },
}

impl fmt::Display for UpdateError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io { message } | Self::Parse { message } | Self::Lock { message } => {
        f.write_str(message)
      }
    }
  }
}

impl Error for UpdateError {}

/// 版本文件插件：识别文件形态并保格式更新其中的版本号（无状态，静态链跨线程共享）
pub trait VersionFilePlugin: Sync {
  /// 按相对路径（实际比较 basename）判断是否走本通道
  fn matches(&self, rel_path: &Path) -> bool;
  /// 更新 `path`（绝对路径）指向的文件；`current` / `new` 为当前与新版本号；
  /// `rel_path` 为用户清单中的原始相对路径，仅用于错误消息文案（与迁移前一致）
  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
  ) -> Result<UpdateOutcome, UpdateError>;
}

/// 内置有序链（ADR-0004：JsManifest → CargoToml → Text 兜底）
static PLUGINS: &[&dyn VersionFilePlugin] = &[
  &js_manifest::JsManifestPlugin,
  &cargo_toml::CargoTomlPlugin,
  &text::TextPlugin,
];

/// 按 `rel_path` 分发到首个命中的插件，更新 `abs_path` 指向的文件
pub fn update_file(
  rel_path: &Path,
  abs_path: &Path,
  current: &str,
  new: &str,
) -> Result<UpdateOutcome, UpdateError> {
  PLUGINS
    .iter()
    .find(|p| p.matches(rel_path))
    .expect("TextPlugin 兜底必命中")
    .update(abs_path, rel_path, current, new)
}

pub(crate) fn read_text(path: &Path, rel_path: &Path) -> Result<String, UpdateError> {
  std::fs::read_to_string(path).map_err(|e| UpdateError::Io {
    message: format!("读取 {} 失败：{e}", rel_path.display()),
  })
}

pub(crate) fn write_text(path: &Path, rel_path: &Path, content: &str) -> Result<(), UpdateError> {
  std::fs::write(path, content).map_err(|e| UpdateError::Io {
    message: format!("写入 {} 失败：{e}", rel_path.display()),
  })
}
