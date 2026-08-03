#![deny(clippy::all)]

//! napi 导出面（ADR-0014 收缩、ADR-0016 再收缩后）：编排 `bumpVersion`、平台
//! Release 四导出、CLI 单入口 `cliRun`。上游 parity 面（versionBump 系、
//! loadBumpConfig）与 changelog 系函数已收归 Rust 内部，不再导出。

use std::path::PathBuf;

use napi_derive::napi;
use serde_json::{Map, Value};

fn resolve_cwd(cwd: Option<String>) -> napi::Result<PathBuf> {
  match cwd {
    Some(c) => Ok(PathBuf::from(c)),
    None => Ok(std::env::current_dir()?),
  }
}

fn to_napi_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::from_reason(e.to_string())
}

// ---------------------------------------------------------------------------
// CLI 单入口（ADR-0016：argv 语法唯一归属 Rust，Node 仅 argv 透传）
// ---------------------------------------------------------------------------

pub struct CliRunTask {
  argv: Vec<String>,
  provider: Option<String>,
}

#[napi]
impl napi::Task for CliRunTask {
  type Output = i32;
  type JsValue = i32;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    Ok(bumpp_core::cli::run_from_argv(
      &self.argv,
      self.provider.as_deref(),
    ))
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

/// CLI：argv 全权交 Rust 解析执行，返回退出码由调用壳回写 `process.exitCode`；
/// `provider` 为平台变体身份（`@vill-v/bumpp-github` 等变体 bin 经位置参数注入）
#[napi]
pub fn cli_run(
  argv: Vec<String>,
  provider: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<CliRunTask> {
  napi::bindgen_prelude::AsyncTask::new(CliRunTask { argv, provider })
}

// ---------------------------------------------------------------------------
// bumpVersion 编排（ADR-0014：JS bump.ts 的 Rust 收编）
// ---------------------------------------------------------------------------

/// 上游 `operation.state` 的形状
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

/// changelog 生成结果
#[napi(object)]
pub struct GenerateChangelogResult {
  pub markdown: String,
  #[napi(js_name = "changelogMD")]
  pub changelog_md: String,
}

/// `bumpVersion` 返回（ADR-0014 收缩后的 `BumpVersion` 形状）
#[napi(object)]
pub struct BumpVersionResult {
  pub bumpp: BumpState,
  pub changelog: Option<GenerateChangelogResult>,
}

pub struct BumpVersionTask {
  overrides: Option<Map<String, Value>>,
  provider: Option<String>,
  cwd: Option<String>,
}

#[napi]
impl napi::Task for BumpVersionTask {
  type Output = BumpVersionResult;
  type JsValue = BumpVersionResult;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let provider = self
      .provider
      .as_deref()
      .map(|p| {
        bumpp_core::release::Provider::parse(p).ok_or_else(|| {
          napi::Error::from_reason(format!(
            "未知 provider: {p}（可用 github / gitlab / gitee / gitcode）"
          ))
        })
      })
      .transpose()?;
    let outcome = bumpp_core::orchestrate::bump_version(
      &bumpp_core::orchestrate::BumpVersionOptions {
        overrides: self.overrides.take(),
        provider,
      },
      &resolve_cwd(self.cwd.take())?,
    )
    .map_err(to_napi_err)?;
    Ok(BumpVersionResult {
      bumpp: outcome.state.into(),
      changelog: outcome.changelog.map(|c| GenerateChangelogResult {
        markdown: c.markdown,
        changelog_md: c.changelog_md,
      }),
    })
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

/// 完整 bump 编排：统一配置解析 → 交互选版本 → changelog → 文件/脚本/git →
/// 可选平台 Release（`provider` 传 'github' | 'gitlab' | 'gitee' | 'gitcode' 时）
#[napi]
pub fn bump_version(
  overrides: Option<Map<String, Value>>,
  provider: Option<String>,
  cwd: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<BumpVersionTask> {
  napi::bindgen_prelude::AsyncTask::new(BumpVersionTask {
    overrides,
    provider,
    cwd,
  })
}

// ---------------------------------------------------------------------------
// 平台 Release 四导出（ADR-0014：per-provider 1:1 parity；共享实现在 Rust 内部）
// ---------------------------------------------------------------------------

/// `createXRelease` 入参的 `bumpp` 槽（TS 结构类型：完整 BumpState 可赋值）
#[napi(object)]
pub struct CreateReleaseBump {
  #[napi(js_name = "newVersion")]
  pub new_version: String,
}

/// `createXRelease` 入参的 `changelog` 槽
#[napi(object)]
pub struct CreateReleaseChangelog {
  pub markdown: Option<String>,
}

/// `createXRelease` 入参（ADR-0014 收缩后的 `BumpVersion` 形状）：
/// token / repo / host 由 Rust 内部解析，不经 JS 传入
#[napi(object)]
pub struct CreateReleaseOptions {
  pub bumpp: CreateReleaseBump,
  pub changelog: Option<CreateReleaseChangelog>,
}

pub struct CreateReleaseTask {
  provider: bumpp_core::release::Provider,
  new_version: String,
  markdown: String,
  cwd: Option<String>,
}

#[napi]
impl napi::Task for CreateReleaseTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    bumpp_core::release::create_release(
      self.provider,
      &self.new_version,
      &self.markdown,
      &resolve_cwd(self.cwd.take())?,
      None,
    )
    .map_err(to_napi_err)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

fn create_release_task(
  provider: bumpp_core::release::Provider,
  options: CreateReleaseOptions,
  cwd: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<CreateReleaseTask> {
  napi::bindgen_prelude::AsyncTask::new(CreateReleaseTask {
    provider,
    new_version: options.bumpp.new_version,
    markdown: options
      .changelog
      .and_then(|c| c.markdown)
      .unwrap_or_default(),
    cwd,
  })
}

/// 创建 GitHub release（token 链：存储 → GH_TOKEN → GITHUB_TOKEN → gh CLI）
#[napi]
pub fn create_github_release(
  options: CreateReleaseOptions,
  cwd: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<CreateReleaseTask> {
  create_release_task(bumpp_core::release::Provider::Github, options, cwd)
}

/// 创建 GitLab release（token 链：存储 → GITLAB_TOKEN；host 经配置 gitlab.host）
#[napi]
pub fn create_gitlab_release(
  options: CreateReleaseOptions,
  cwd: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<CreateReleaseTask> {
  create_release_task(bumpp_core::release::Provider::Gitlab, options, cwd)
}

/// 创建 Gitee release（token 链：存储 → GITEE_TOKEN）
#[napi]
pub fn create_gitee_release(
  options: CreateReleaseOptions,
  cwd: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<CreateReleaseTask> {
  create_release_task(bumpp_core::release::Provider::Gitee, options, cwd)
}

/// 创建 GitCode release（token 链：存储 → GITCODE_TOKEN）
#[napi]
pub fn create_gitcode_release(
  options: CreateReleaseOptions,
  cwd: Option<String>,
) -> napi::bindgen_prelude::AsyncTask<CreateReleaseTask> {
  create_release_task(bumpp_core::release::Provider::Gitcode, options, cwd)
}
