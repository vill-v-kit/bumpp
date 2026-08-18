#![deny(clippy::all)]

//! napi 导出面（收缩、 再收缩、 三收缩后）：编排
//! `bumpVersion`、CLI 单入口 `cliRun`。平台 Release 四导出已删——独立
//! release 由 CLI `vbumpp release` 子命令承接；上游 parity 面
//! （versionBump 系、loadBumpConfig）与 changelog 系函数收归 Rust 内部。
//! `bumpVersion` 入参为类型化边界结构体（见 `config` 模块）。

pub mod config;

use std::env;
use std::fmt::Display;
use std::path::PathBuf;

use napi::bindgen_prelude::AsyncTask;
use napi::{Env, Error, Result, Task};
use napi_derive::napi;

use crate::config::BumpConfig;
use vbumpp_core::info;
use vbumpp_core::{cli, orchestrate, release};

fn resolve_cwd(cwd: Option<String>) -> Result<PathBuf> {
  match cwd {
    Some(c) => Ok(PathBuf::from(c)),
    None => Ok(env::current_dir()?),
  }
}

fn to_napi_err(e: impl Display) -> Error {
  Error::from_reason(e.to_string())
}

// ---------------------------------------------------------------------------
// CLI 单入口（argv 语法唯一归属 Rust，Node 仅 argv 透传）
// ---------------------------------------------------------------------------

pub struct CliRunTask {
  argv: Vec<String>,
  provider: Option<String>,
}

#[napi]
impl Task for CliRunTask {
  type Output = i32;
  type JsValue = i32;

  fn compute(&mut self) -> Result<Self::Output> {
    Ok(cli::run_from_argv(&self.argv, self.provider.as_deref()))
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(output)
  }
}

/// CLI：argv 全权交 Rust 解析执行，返回退出码由调用壳回写 `process.exitCode`；
/// `provider` 为平台变体身份（`@vill-v/bumpp-github` 等变体 bin 经位置参数注入）
#[napi]
pub fn cli_run(argv: Vec<String>, provider: Option<String>) -> AsyncTask<CliRunTask> {
  AsyncTask::new(CliRunTask { argv, provider })
}

// ---------------------------------------------------------------------------
// bumpVersion 编排（JS bump.ts 的 Rust 收编）
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

impl From<info::BumpState> for BumpState {
  fn from(s: info::BumpState) -> Self {
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

/// `bumpVersion` 返回（收缩后的 `BumpVersion` 形状）
#[napi(object)]
pub struct BumpVersionResult {
  pub bumpp: BumpState,
  pub changelog: Option<GenerateChangelogResult>,
}

pub struct BumpVersionTask {
  overrides: Option<BumpConfig>,
  provider: Option<String>,
  cwd: Option<String>,
}

#[napi]
impl Task for BumpVersionTask {
  type Output = BumpVersionResult;
  type JsValue = BumpVersionResult;

  fn compute(&mut self) -> Result<Self::Output> {
    let provider = self
      .provider
      .as_deref()
      .map(|p| {
        release::Provider::parse(p).ok_or_else(|| {
          Error::from_reason(format!(
            "unknown provider: {p} (expected github / gitlab / gitee / gitcode)"
          ))
        })
      })
      .transpose()?;
    // 类型化边界产物转合并层载体（结构体是校验载体，Map 是合并载体）
    let overrides = self.overrides.take().map(BumpConfig::into_map);
    let outcome = orchestrate::bump_version(
      &orchestrate::BumpVersionOptions {
        overrides,
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

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(output)
  }
}

/// 完整 bump 编排：统一配置解析 → 交互选版本 → changelog → 文件/脚本/git →
/// 可选平台 Release（`provider` 传 'github' | 'gitlab' | 'gitee' | 'gitcode' 时）。
/// `overrides` 为类型化配置覆盖（类型不符在边界即运行期报错）
#[napi]
pub fn bump_version(
  overrides: Option<BumpConfig>,
  provider: Option<String>,
  cwd: Option<String>,
) -> AsyncTask<BumpVersionTask> {
  AsyncTask::new(BumpVersionTask {
    overrides,
    provider,
    cwd,
  })
}
