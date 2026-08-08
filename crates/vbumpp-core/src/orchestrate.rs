//! bumpVersion 编排（ADR-0014）：原 JS `bump.ts` 职责的 Rust 收编——
//! 统一配置解析 → 最近 tag → 版本确定（配置 `release` 键直选或交互菜单，
//! COL-60）→ changelog（有 tag 才生成）→ versionBump → 可选平台 Release。
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
  // ---- 统一配置解析（四层合并：内建默认 ← 全局 ← 项目 ← overrides，ADR-0013）----
  let mut merged = crate::config::load_bump_config(options.overrides.clone(), cwd)?;

  // ---- 版本确定三键（COL-60）：release / preid / currentVersion 自 merged 配置
  // 穿入 version_bump_info——release 携带即非交互（release type / 版本号直选），
  // 缺省或 "prompt" 走交互菜单；preid / currentVersion 缺它则 pre* 释放算错
  // 标识、新版本基线仍来自文件探测
  let release = config_str(&merged, "release")?;
  let preid = config_str(&merged, "preid")?;
  let current_version = config_str(&merged, "currentVersion")?;

  // ---- confirm 门控（COL-60）：交互选定版本即确认，不再二次问（文档语义：
  // 「命令行交互选定版本后不会再问」）；非交互路径按 merged confirm 执行——
  // 缺省 true 二次确认（上游语义），CI / 脚本场景配 confirm = false ----
  let interactive = release.as_deref().is_none_or(|r| r == "prompt");
  if interactive {
    merged.insert("confirm".into(), Value::Bool(false));
  }

  // ---- 最近 tag（真实 tag 名；无 tag / 非 git 仓库软失败 None）----
  let current_tag = crate::git::get_last_git_tag(cwd)?;

  // ---- 版本确定（JS：`versionBumpInfo()`）----
  let state = crate::info::version_bump_info(
    &crate::info::BumpInfoOptions {
      release: release.as_deref(),
      files: &[],
      current_version: current_version.as_deref(),
      preid: preid.as_deref(),
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

  // ---- versionBump（merged + release 固定为已确定的新版本）----
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

/// 字符串型配置键提取（COL-60）：缺省 / null 为 None；空串与非字符串值报错——
/// 配置写错不允许静默回落（release 空串/错类型若静默当缺省会意外弹交互菜单；
/// 空串报错对齐上游 `release: ""` 经 loose 解析抛错的行为）
fn config_str(merged: &Map<String, Value>, key: &str) -> Result<Option<String>, OrchestrateError> {
  match merged.get(key) {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(s)) if !s.is_empty() => Ok(Some(s.clone())),
    Some(_) => Err(OrchestrateError::Config {
      message: format!(
        "config key \"{key}\" must be a non-empty string — fix the value or remove the key"
      ),
    }),
  }
}
