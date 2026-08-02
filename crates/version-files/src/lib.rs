//! version-files：版本文件的解析与更新插件底座（ADR-0004）。
//!
//! 内置静态插件链按 `matches` 顺序分发，命中即走对应通道：
//! `JsManifestPlugin`（JS 生态 JSON manifest）→ `TextPlugin`（文本模板替换，兜底）；
//! `CargoTomlPlugin` 链位预留（COL-23 落地）。
//! 静态分发，无运行时 registry。

use std::error::Error;
use std::fmt;
use std::path::Path;

mod js_manifest;
mod text;

/// 单次文件更新结果（对应 bumpp-core 编排层的 FileUpdated / FileSkipped 事件来源）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
  Updated,
  Skipped,
}

#[derive(Debug)]
pub enum UpdateError {
  Io { message: String },
}

impl fmt::Display for UpdateError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io { message } => f.write_str(message),
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

/// 内置有序链（CargoTomlPlugin 预留于 JsManifest 之后、Text 之前——COL-23）
static PLUGINS: &[&dyn VersionFilePlugin] = &[&js_manifest::JsManifestPlugin, &text::TextPlugin];

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
