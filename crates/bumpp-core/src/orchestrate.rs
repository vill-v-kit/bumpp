//! bumpVersion 编排（ADR-0014）：原 JS `bump.ts` 职责的 Rust 收编——
//! 统一配置解析（confirm 缺省关闭）→ 最近 tag → 交互选版本 → changelog
//! （有 tag 才生成）→ versionBump → 可选平台 Release。
//! spinner 动画以进度打印替代（ADR-0002）；明文 token 不出 release 模块。

use std::error::Error;
use std::fmt;
use std::path::Path;

use serde_json::{Map, Value};

use crate::bump::BumpOptions;
use crate::changelog::GenerateChangelogOutcome;
use crate::info::BumpState;
use crate::release::Provider;

/// 编排入参：`overrides` 为扁平全量配置覆盖（与配置文件同形，含 `changelog` /
/// `gitlab` 段）；`provider` 缺省仅 bump，传值则 bump 完成后接平台 Release
#[derive(Debug, Clone, Default)]
pub struct BumpVersionOptions {
  pub overrides: Option<Map<String, Value>>,
  pub provider: Option<Provider>,
}

/// 编排产出（对齐 JS `BumpVersion` 收缩后的形状，ADR-0014）
#[derive(Debug)]
pub struct BumpVersionOutcome {
  pub state: BumpState,
  pub changelog: Option<GenerateChangelogOutcome>,
}

#[derive(Debug)]
pub enum OrchestrateError {
  Config { message: String },
  Git { message: String },
  Info { message: String },
  Changelog { message: String },
  Bump { message: String },
  Release { message: String },
}

impl fmt::Display for OrchestrateError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Config { message }
      | Self::Git { message }
      | Self::Info { message }
      | Self::Changelog { message }
      | Self::Bump { message }
      | Self::Release { message } => f.write_str(message),
    }
  }
}

impl Error for OrchestrateError {}

impl From<crate::config::LoadConfigError> for OrchestrateError {
  fn from(e: crate::config::LoadConfigError) -> Self {
    Self::Config {
      message: e.to_string(),
    }
  }
}

impl From<crate::exec::ExecError> for OrchestrateError {
  fn from(e: crate::exec::ExecError) -> Self {
    Self::Git {
      message: e.to_string(),
    }
  }
}

impl From<crate::info::InfoError> for OrchestrateError {
  fn from(e: crate::info::InfoError) -> Self {
    Self::Info {
      message: e.to_string(),
    }
  }
}

impl From<crate::changelog::ChangelogError> for OrchestrateError {
  fn from(e: crate::changelog::ChangelogError) -> Self {
    Self::Changelog {
      message: e.to_string(),
    }
  }
}

impl From<crate::bump::BumpError> for OrchestrateError {
  fn from(e: crate::bump::BumpError) -> Self {
    Self::Bump {
      message: e.to_string(),
    }
  }
}

impl From<crate::release::ReleaseError> for OrchestrateError {
  fn from(e: crate::release::ReleaseError) -> Self {
    Self::Release {
      message: e.to_string(),
    }
  }
}

/// 完整 bump 编排（原 JS `bumpVersion` + `bumpVersionWithBaseRelease` 的统一形态）
pub fn bump_version(
  options: &BumpVersionOptions,
  cwd: &Path,
) -> Result<BumpVersionOutcome, OrchestrateError> {
  // ---- 统一配置解析：confirm 缺省关闭（版本经交互选择，不再二次确认；
  // JS parity：`loadBumpConfig({ confirm: false, ...option })`）----
  let mut overrides = options.overrides.clone().unwrap_or_default();
  overrides
    .entry("confirm".to_owned())
    .or_insert(Value::Bool(false));
  let merged = crate::config::load_bump_config(Some(overrides), cwd)?;

  // ---- 最近 tag（真实 tag 名；无 tag / 非 git 仓库软失败 None）----
  let current_tag = crate::git::get_last_git_tag(cwd)?;

  // ---- 交互选版本（JS：`versionBumpInfo()` 无参调用）----
  let state = crate::info::version_bump_info(
    &crate::info::BumpInfoOptions {
      release: None,
      files: &[],
      current_version: None,
      preid: None,
    },
    cwd,
  )?;

  // ---- changelog：存在 tag 才生成（spinner → 进度打印）----
  let changelog = match current_tag {
    Some(tag) => {
      let outcome = crate::changelog::generate_changelog(
        &crate::changelog::GenerateChangelogOptions {
          overrides: options.overrides.clone(),
          from: tag,
          to: state.new_version.clone(),
        },
        cwd,
      )?;
      println!(
        "{} Update {} success",
        dialoguer::console::style("✔").green(),
        outcome.output
      );
      Some(outcome)
    }
    None => None,
  };

  // ---- versionBump（merged + release 固定为交互选定的新版本）----
  let bump_options = BumpOptions::from_merged(&merged, &state.new_version);
  crate::bump::version_bump(&bump_options, cwd, &mut |_| {})?;

  // ---- 平台 Release（spinner → 进度打印）----
  if let Some(provider) = options.provider {
    let markdown = changelog
      .as_ref()
      .map(|c| c.markdown.as_str())
      .unwrap_or("");
    crate::release::create_release(
      provider,
      &state.new_version,
      markdown,
      cwd,
      options.overrides.as_ref(),
    )?;
    println!(
      "{} [{}] add release v{} success",
      dialoguer::console::style("✔").green(),
      provider.display(),
      state.new_version
    );
  }

  Ok(BumpVersionOutcome { state, changelog })
}
