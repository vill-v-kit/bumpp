//! JavaScript 生态版本能力（上游 bumpp v11 `updateManifestFile` 纯迁移，ADR-0007/0010）：
//! JSONC 容错解析后仅替换 `version` 值所在的文本区间（package-lock 另含
//! `packages[""].version`），其余字节原样保留。

use std::path::Path;

use jsonc_parser::ast::{Object, Value};
use jsonc_parser::common::{Range, Ranged};
use jsonc_parser::{parse_to_ast, CollectOptions, ParseOptions};

use super::super::recursive::javascript::MANIFEST_BASENAMES;
use super::super::{read_text, write_text, FilesError, UpdateOutcome};
use crate::jsonc::{get_prop, is_manifest};

/// manifest 通道识别：basename 小写比较命中清单常量表
pub(crate) fn matches(rel_path: &Path) -> bool {
  let basename = rel_path
    .file_name()
    .map(|n| n.to_string_lossy().trim().to_lowercase())
    .unwrap_or_default();
  MANIFEST_BASENAMES.contains(&basename.as_str())
}

/// 版本解析（ADR-0009）：上游 `readVersion` 的提取部分——JSONC 容错解析 +
/// is_manifest 门 + `version` 字段取字符串；semver 校验由编排层统一承担
pub(crate) fn read_version(path: &Path) -> Option<String> {
  let text = std::fs::read_to_string(path).ok()?;
  let Value::Object(root) = crate::jsonc::parse(&text)? else {
    return None;
  };
  if !is_manifest(&root) {
    return None;
  }
  Some(
    get_prop(&root, "version")?
      .value
      .as_string_lit()?
      .value
      .to_string(),
  )
}

/// 上游 `updateManifestFile`：只设置顶层 `version`（package-lock 另含嵌套路径），
/// 通过文本区间替换保留原格式
pub(crate) fn update(
  path: &Path,
  rel_path: &Path,
  _current: &str,
  new: &str,
) -> Result<UpdateOutcome, FilesError> {
  let text = read_text(path, rel_path)?;
  // 上游 jsonc.parse 容错：解析失败的文件按 skip 处理，批次继续
  let Ok(ast) = parse_to_ast(&text, &CollectOptions::default(), &ParseOptions::default()) else {
    return Ok(UpdateOutcome::Skipped);
  };
  let Some(Value::Object(root)) = &ast.value else {
    return Ok(UpdateOutcome::Skipped);
  };
  if !is_manifest(root) {
    return Ok(UpdateOutcome::Skipped);
  }
  // version 缺失或为 null → 跳过
  let Some(version_prop) = get_prop(root, "version") else {
    return Ok(UpdateOutcome::Skipped);
  };
  if matches!(&version_prop.value, Value::NullKeyword(_)) {
    return Ok(UpdateOutcome::Skipped);
  }
  // version 已是新值 → 跳过（不重写文件）
  if let Value::StringLit(s) = &version_prop.value {
    if s.value == new {
      return Ok(UpdateOutcome::Skipped);
    }
  }

  let mut edits = vec![(version_prop.value.range(), quote(new))];
  // isPackageLockManifest：packages[""].version 为 string 时一并更新
  if let Some(nested) = package_lock_root_version(root) {
    edits.push((nested.range(), quote(new)));
  }

  write_text(path, rel_path, &apply_edits(&text, &edits))?;
  Ok(UpdateOutcome::Updated)
}

/// 上游 `isPackageLockManifest` 的取值：`packages[""].version` 为 string 时返回该节点
fn package_lock_root_version<'a>(root: &'a Object<'a>) -> Option<&'a Value<'a>> {
  let version = get_prop(root, "packages")
    .and_then(|p| p.value.as_object())
    .and_then(|p| get_prop(p, ""))
    .and_then(|p| p.value.as_object())
    .and_then(|p| get_prop(p, "version"))
    .map(|p| &p.value)?;
  matches!(version, Value::StringLit(_)).then_some(version)
}

fn quote(value: &str) -> String {
  format!("\"{value}\"")
}

/// 将区间替换应用到原文（按起点倒序，避免位移）
fn apply_edits(text: &str, edits: &[(Range, String)]) -> String {
  let mut sorted: Vec<_> = edits.to_vec();
  sorted.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
  let mut result = text.to_owned();
  for (range, replacement) in sorted {
    result.replace_range(range.start..range.end, &replacement);
  }
  result
}
