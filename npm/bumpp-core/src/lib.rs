#![deny(clippy::all)]

use std::path::PathBuf;

use napi_derive::napi;
use serde_json::{Map, Value};

/// Scaffold smoke-test export: proves the Rust → napi → JS link works end to end.
/// Real bumpp APIs land in later tickets (COL-8+).
#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

fn resolve_cwd(cwd: Option<String>) -> napi::Result<PathBuf> {
  match cwd {
    Some(c) => Ok(PathBuf::from(c)),
    None => Ok(std::env::current_dir()?),
  }
}

/// 加载并合并 bumpp 配置（仅 JSON 配置文件），语义对齐上游 bumpp v11 的 `loadBumpConfig`。
#[napi]
pub fn load_bump_config(
  overrides: Option<Map<String, Value>>,
  cwd: Option<String>,
) -> napi::Result<Map<String, Value>> {
  bumpp_core::config::load_bump_config(overrides, &resolve_cwd(cwd)?)
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// 文件版本更新结果（对齐上游 operation.state 的 updatedFiles / skippedFiles）
#[napi(object)]
pub struct UpdateFilesOutcome {
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  #[napi(js_name = "skippedFiles")]
  pub skipped_files: Vec<String>,
}

/// 更新文件中的版本号（manifest 保格式更新 + 文本模板替换），对齐上游 `updateFiles`。
#[napi]
pub fn update_files(
  files: Vec<String>,
  cwd: Option<String>,
  current_version: String,
  new_version: String,
) -> napi::Result<UpdateFilesOutcome> {
  let cwd = resolve_cwd(cwd)?;
  bumpp_core::files::update_files(&files, &cwd, &current_version, &new_version)
    .map(|o| UpdateFilesOutcome {
      updated_files: o.updated_files().iter().map(|s| s.to_string()).collect(),
      skipped_files: o.skipped_files().iter().map(|s| s.to_string()).collect(),
    })
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}
