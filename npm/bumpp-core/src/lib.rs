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

fn to_napi_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::from_reason(e.to_string())
}

/// 加载并合并 bumpp 配置（仅 JSON 配置文件），语义对齐上游 bumpp v11 的 `loadBumpConfig`。
#[napi]
pub fn load_bump_config(
  overrides: Option<Map<String, Value>>,
  cwd: Option<String>,
) -> napi::Result<Map<String, Value>> {
  bumpp_core::config::load_bump_config(overrides, &resolve_cwd(cwd)?).map_err(to_napi_err)
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
    .map_err(to_napi_err)
}

#[napi(object)]
pub struct CommitSpec {
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  pub all: bool,
  #[napi(js_name = "noVerify")]
  pub no_verify: bool,
  pub sign: bool,
  pub message: String,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
}

#[napi(object)]
pub struct TagSpec {
  pub name: String,
  pub message: String,
  pub sign: bool,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
}

#[napi(object)]
pub struct GitCommitOutcome {
  pub event: String,
  #[napi(js_name = "commitMessage")]
  pub commit_message: String,
}

#[napi(object)]
pub struct GitTagOutcome {
  pub event: String,
  #[napi(js_name = "tagName")]
  pub tag_name: String,
}

#[napi(object)]
pub struct GitPushOutcome {
  pub event: String,
}

#[napi(object)]
pub struct NpmScriptOutcome {
  pub event: String,
  pub script: String,
}

/// git commit（shell out 到 git 二进制），对齐上游 `gitCommit`。
#[napi]
pub fn git_commit(cwd: Option<String>, spec: CommitSpec) -> napi::Result<GitCommitOutcome> {
  let core_spec = bumpp_core::git::CommitSpec {
    updated_files: &spec.updated_files,
    all: spec.all,
    no_verify: spec.no_verify,
    sign: spec.sign,
    message: &spec.message,
    new_version: &spec.new_version,
  };
  bumpp_core::git::git_commit(&resolve_cwd(cwd)?, &core_spec)
    .map(|(e, m)| GitCommitOutcome {
      event: e.as_str().to_string(),
      commit_message: m,
    })
    .map_err(to_napi_err)
}

/// git tag（附注），对齐上游 `gitTag`。
#[napi]
pub fn git_tag(cwd: Option<String>, spec: TagSpec) -> napi::Result<GitTagOutcome> {
  let core_spec = bumpp_core::git::TagSpec {
    name: &spec.name,
    message: &spec.message,
    sign: spec.sign,
    new_version: &spec.new_version,
  };
  bumpp_core::git::git_tag(&resolve_cwd(cwd)?, &core_spec)
    .map(|(e, n)| GitTagOutcome {
      event: e.as_str().to_string(),
      tag_name: n,
    })
    .map_err(to_napi_err)
}

/// git push（withTags 时追加 `git push --tags`），对齐上游 `gitPush`。
#[napi]
pub fn git_push(cwd: Option<String>, with_tags: bool) -> napi::Result<GitPushOutcome> {
  bumpp_core::git::git_push(&resolve_cwd(cwd)?, with_tags)
    .map(|e| GitPushOutcome {
      event: e.as_str().to_string(),
    })
    .map_err(to_napi_err)
}

/// 执行 package.json 中的 npm script（ignoreScripts 时不执行），对齐上游 `runNpmScript`。
/// 返回 null 表示未执行；脚本非零退出不传播（上游 parity）。
#[napi]
pub fn run_npm_script(
  cwd: Option<String>,
  script: String,
  ignore_scripts: bool,
) -> napi::Result<Option<NpmScriptOutcome>> {
  bumpp_core::scripts::run_npm_script(&resolve_cwd(cwd)?, &script, ignore_scripts)
    .map(|r| {
      r.map(|(e, s)| NpmScriptOutcome {
        event: e.as_str().to_string(),
        script: s,
      })
    })
    .map_err(to_napi_err)
}
