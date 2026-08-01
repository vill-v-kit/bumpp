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

#[napi(object)]
#[derive(Default)]
pub struct BumpInfoArg {
  pub release: Option<String>,
  pub files: Option<Vec<String>>,
  pub cwd: Option<String>,
  pub preid: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: Option<String>,
}

/// 上游 operation.state 的形状
#[napi(object)]
pub struct BumpState {
  pub release: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: String,
  #[napi(js_name = "currentVersionSource")]
  pub current_version_source: String,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
  #[napi(js_name = "commitMessage")]
  pub commit_message: String,
  #[napi(js_name = "tagName")]
  pub tag_name: String,
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  #[napi(js_name = "skippedFiles")]
  pub skipped_files: Vec<String>,
}

impl From<bumpp_core::info::BumpState> for BumpState {
  fn from(s: bumpp_core::info::BumpState) -> Self {
    Self {
      release: s.release,
      current_version: s.current_version,
      current_version_source: s.current_version_source,
      new_version: s.new_version,
      commit_message: s.commit_message,
      tag_name: s.tag_name,
      updated_files: s.updated_files,
      skipped_files: s.skipped_files,
    }
  }
}

/// 上游 versionBumpInfo 的返回形状：{ state }
#[napi(object)]
pub struct VersionBumpInfo {
  pub state: BumpState,
}

pub struct VersionBumpInfoTask {
  arg: Option<napi::Either<String, BumpInfoArg>>,
}

#[napi]
impl napi::Task for VersionBumpInfoTask {
  type Output = VersionBumpInfo;
  type JsValue = VersionBumpInfo;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let arg = match self.arg.take() {
      // 上游：字符串入参等价于 { release: arg }
      Some(napi::Either::A(release)) => BumpInfoArg {
        release: Some(release),
        ..Default::default()
      },
      Some(napi::Either::B(a)) => a,
      None => BumpInfoArg::default(),
    };
    let cwd = resolve_cwd(arg.cwd)?;
    let files = arg.files.unwrap_or_default();
    let options = bumpp_core::info::BumpInfoOptions {
      release: arg.release.as_deref(),
      files: &files,
      current_version: arg.current_version.as_deref(),
      preid: arg.preid.as_deref(),
    };
    bumpp_core::info::version_bump_info(&options, &cwd)
      .map(|s| VersionBumpInfo { state: s.into() })
      .map_err(to_napi_err)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

/// 计算 bump 信息（当前版本 + 新版本），必要时在 Rust 侧渲染交互 prompt。
/// 对齐上游 bumpp v11 的 `versionBumpInfo`。
#[napi]
pub fn version_bump_info(
  arg: Option<napi::Either<String, BumpInfoArg>>,
) -> napi::bindgen_prelude::AsyncTask<VersionBumpInfoTask> {
  napi::bindgen_prelude::AsyncTask::new(VersionBumpInfoTask { arg })
}
