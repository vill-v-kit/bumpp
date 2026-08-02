//! 文件版本更新：对齐上游 bumpp v11 `updateFile` / `updateManifestFile` / `updateTextFile`。
//!
//! - manifest（package.json 等 8 种 basename）：JSONC 容错解析后仅替换 `version` 值所在的
//!   文本区间（package-lock 另含 `packages[""].version`），其余字节原样保留；
//! - 其他文件：按上游正则 `(\b|v){version}\b` 全局替换（`\b` 为 JS 的 ASCII 语义）。

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use jsonc_parser::ast::{Object, Value};
use jsonc_parser::common::{Range, Ranged};
use jsonc_parser::{parse_to_ast, CollectOptions, ParseOptions};

use crate::jsonc::{get_prop, is_manifest};
use crate::progress::ProgressEvent;

/// 按 manifest 处理的 basename（上游 switch 列表，小写比较）
const MANIFEST_BASENAMES: [&str; 8] = [
  "package.json",
  "package-lock.json",
  "bower.json",
  "component.json",
  "jsr.json",
  "jsr.jsonc",
  "deno.json",
  "deno.jsonc",
];

#[derive(Debug)]
pub enum FilesError {
  Io { message: String },
}

impl fmt::Display for FilesError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io { message } => f.write_str(message),
    }
  }
}

impl Error for FilesError {}

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

/// 上游 `updateFiles`：逐个文件更新版本号，按处理顺序产出 FileUpdated / FileSkipped 事件
pub fn update_files(
  files: &[String],
  cwd: &Path,
  current_version: &str,
  new_version: &str,
) -> Result<UpdateFilesOutcome, FilesError> {
  let mut outcome = UpdateFilesOutcome::default();
  for rel_path in files {
    let modified = update_file(rel_path, cwd, current_version, new_version)?;
    // 上游事件路径经 path.resolve(cwd, relPath) 归一化（消除 ./ 与 .. 段）
    let abs_path = resolve(cwd, rel_path).to_string_lossy().into_owned();
    let event = if modified {
      ProgressEvent::FileUpdated
    } else {
      ProgressEvent::FileSkipped
    };
    outcome.events.push((event, abs_path));
  }
  Ok(outcome)
}

/// 上游 `updateFile`：文件不存在 → skipped；按 basename 分流 manifest / text
fn update_file(
  rel_path: &str,
  cwd: &Path,
  current_version: &str,
  new_version: &str,
) -> Result<bool, FilesError> {
  let path = cwd.join(rel_path);
  if !path.exists() {
    return Ok(false);
  }
  let basename = path
    .file_name()
    .map(|n| n.to_string_lossy().trim().to_lowercase())
    .unwrap_or_default();
  if MANIFEST_BASENAMES.contains(&basename.as_str()) {
    update_manifest(&path, rel_path, new_version)
  } else {
    update_text(&path, rel_path, current_version, new_version)
  }
}

/// 上游 `updateManifestFile`：只设置顶层 `version`（package-lock 另含嵌套路径），
/// 通过文本区间替换保留原格式
fn update_manifest(path: &Path, rel_path: &str, new_version: &str) -> Result<bool, FilesError> {
  let text = read_text(path, rel_path)?;
  // 上游 jsonc.parse 容错：解析失败的文件按 skip 处理，批次继续
  let Ok(ast) = parse_to_ast(&text, &CollectOptions::default(), &ParseOptions::default()) else {
    return Ok(false);
  };
  let Some(Value::Object(root)) = &ast.value else {
    return Ok(false);
  };
  if !is_manifest(root) {
    return Ok(false);
  }
  // version 缺失或为 null → 跳过
  let Some(version_prop) = get_prop(root, "version") else {
    return Ok(false);
  };
  if matches!(&version_prop.value, Value::NullKeyword(_)) {
    return Ok(false);
  }
  // version 已是新值 → 跳过（不重写文件）
  if let Value::StringLit(s) = &version_prop.value {
    if s.value == new_version {
      return Ok(false);
    }
  }

  let mut edits = vec![(version_prop.value.range(), quote(new_version))];
  // isPackageLockManifest：packages[""].version 为 string 时一并更新
  if let Some(nested) = package_lock_root_version(root) {
    edits.push((nested.range(), quote(new_version)));
  }

  write_text(path, rel_path, &apply_edits(&text, &edits))?;
  Ok(true)
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

/// 上游 `updateTextFile`：全局替换 `(\b|v){version}\b`，`\b` 对齐 JS 的 ASCII 语义
fn update_text(
  path: &Path,
  rel_path: &str,
  current_version: &str,
  new_version: &str,
) -> Result<bool, FilesError> {
  let text = read_text(path, rel_path)?;
  if !text.contains(current_version) {
    return Ok(false);
  }
  // 上游 sanitizedVersion 转义全部 \W 字符；regex::escape 语义等价
  let pattern = format!("((?-u:\\b)|v){}(?-u:\\b)", regex::escape(current_version));
  let re = regex::Regex::new(&pattern).expect("版本号转义后必为合法正则");
  let new_text = re.replace_all(&text, |caps: &regex::Captures| {
    format!("{}{new_version}", &caps[1])
  });
  write_text(path, rel_path, &new_text)?;
  Ok(true)
}

fn read_text(path: &Path, rel_path: &str) -> Result<String, FilesError> {
  fs::read_to_string(path).map_err(|e| FilesError::Io {
    message: format!("读取 {rel_path} 失败：{e}"),
  })
}

fn write_text(path: &Path, rel_path: &str, content: &str) -> Result<(), FilesError> {
  fs::write(path, content).map_err(|e| FilesError::Io {
    message: format!("写入 {rel_path} 失败：{e}"),
  })
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
