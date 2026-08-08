//! 文本兜底通道的版本能力（上游 bumpp v11 `updateTextFile` 纯迁移，ADR-0007）：
//! 按上游正则 `(\b|v){version}\b` 全局替换（`\b` 为 JS 的 ASCII 语义）。
//! 兜底通道仅有版本更新能力——无清单、无版本解析、无 install 适配。

use std::path::Path;

use super::super::{read_text, write_text, FilesError, UpdateOutcome};

/// 兜底通道：永远命中（对应插件必须处于链尾）
pub(crate) fn matches(_rel_path: &Path) -> bool {
  true
}

/// 上游 `updateTextFile`：全局替换 `(\b|v){version}\b`，`\b` 对齐 JS 的 ASCII 语义
pub(crate) fn update(
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
  let re = regex::Regex::new(&pattern).expect("an escaped version is always a valid regex");
  let new_text = re.replace_all(&text, |caps: &regex::Captures| format!("{}{new}", &caps[1]));
  write_text(path, rel_path, &new_text)?;
  Ok(UpdateOutcome::Updated)
}
