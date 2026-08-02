//! 文本兜底插件（上游 bumpp v11 `updateTextFile` 纯迁移，ADR-0007）：
//! 按上游正则 `(\b|v){version}\b` 全局替换（`\b` 为 JS 的 ASCII 语义）。

use std::path::Path;

use super::{read_text, write_text, Ecosystem, FilesError, UpdateOutcome, VersionFilePlugin};

pub(crate) struct TextPlugin;

impl VersionFilePlugin for TextPlugin {
  /// 兜底通道：永远命中（必须处于链尾）
  fn matches(&self, _rel_path: &Path) -> bool {
    true
  }

  fn ecosystem(&self) -> Option<Ecosystem> {
    None // 兜底通道不归属任何生态
  }

  /// 兜底通道无清单概念
  fn manifest_basenames(&self) -> &'static [&'static str] {
    &[]
  }

  /// 上游 `updateTextFile`：全局替换 `(\b|v){version}\b`，`\b` 对齐 JS 的 ASCII 语义
  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
  ) -> Result<UpdateOutcome, FilesError> {
    let text = read_text(path, rel_path)?;
    if !text.contains(current) {
      return Ok(UpdateOutcome::Skipped);
    }
    // 上游 sanitizedVersion 转义全部 \W 字符；regex::escape 语义等价
    let pattern = format!("((?-u:\\b)|v){}(?-u:\\b)", regex::escape(current));
    let re = regex::Regex::new(&pattern).expect("版本号转义后必为合法正则");
    let new_text = re.replace_all(&text, |caps: &regex::Captures| format!("{}{new}", &caps[1]));
    write_text(path, rel_path, &new_text)?;
    Ok(UpdateOutcome::Updated)
  }
}
