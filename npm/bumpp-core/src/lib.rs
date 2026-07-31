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

/// 加载并合并 bumpp 配置（仅 JSON 配置文件），语义对齐上游 bumpp v11 的 `loadBumpConfig`。
#[napi]
pub fn load_bump_config(
  overrides: Option<Map<String, Value>>,
  cwd: Option<String>,
) -> napi::Result<Map<String, Value>> {
  let cwd = match cwd {
    Some(c) => PathBuf::from(c),
    None => std::env::current_dir()?,
  };
  bumpp_core::config::load_bump_config(overrides, &cwd)
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}
