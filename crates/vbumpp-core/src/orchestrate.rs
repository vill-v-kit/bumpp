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
use crate::config::BumpConfig;
use crate::effects::{Effects, RealEffects};
use crate::info::BumpState;
use crate::release::Provider;

/// 编排入参：`overrides` 为扁平全量配置覆盖（与配置文件同形，含 `changelog` /
/// `gitlab` 段）；`provider` 缺省仅 bump，传值则 bump 完成后接平台 Release
#[derive(Debug, Clone, Default)]
pub struct BumpVersionOptions {
  pub overrides: Option<Map<String, Value>>,
  pub provider: Option<Provider>,
}

/// 编排产出（对齐 JS `BumpVersion` 收缩后的形状，ADR-0014）；
/// `bump` 为 versionBump 的结果（commit message / tag 名 / 逐文件清单——
/// COL-85 dry-run 的 git 动作与判定行数据源）；`config` 为合并配置的
/// 形状解析产物（ADR-0037），dry-run 计划复用它做命令分类
#[derive(Debug)]
pub struct BumpVersionOutcome {
  pub state: BumpState,
  pub changelog: Option<GenerateChangelogOutcome>,
  pub bump: crate::bump::BumpResults,
  pub config: BumpConfig,
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
  bump_version_with(&RealEffects, options, cwd)
}

/// `bump_version` 的效应注入形态：changelog 写盘与提交、versionBump 全链、
/// 平台 Release HTTP 统一经效应边界（预演与执行同路的注入位）
pub fn bump_version_with(
  eff: &dyn Effects,
  options: &BumpVersionOptions,
  cwd: &Path,
) -> Result<BumpVersionOutcome, OrchestrateError> {
  // 成功行打印为显示副作用：与进度行同理回传调用方注入的显示汇
  bump_version_at(eff, options, cwd, &mut |line| println!("{line}"))
}

/// 可测内核：成功行打印经 `display` 汇注入；`bump_version_with` 以 stdout
/// 透传（行为逐字节不变），dry-run 以捕获透传（行进入计划而非直接打印）
pub fn bump_version_at(
  eff: &dyn Effects,
  options: &BumpVersionOptions,
  cwd: &Path,
  display: &mut dyn FnMut(&str),
) -> Result<BumpVersionOutcome, OrchestrateError> {
  // ---- 统一配置解析（四层合并：内建默认 ← 全局 ← 项目 ← overrides，ADR-0013）----
  let merged = crate::config::load_bump_config(options.overrides.clone(), cwd)?;

  // ---- 形状解析（ADR-0037）：merged 一次过结构体，编排与 dry-run 计划
  //（BumpVersionOutcome.config）共用同一份产物；文件层类型校验已拦文件
  // 来源，overrides 层类型不符在此报错 ----
  let mut config =
    crate::config::shape_of(&merged).map_err(|message| OrchestrateError::Config { message })?;

  // ---- 版本确定三键（COL-60）：release / preid / currentVersion 自形状产物
  // 穿入 version_bump_info——release 携带即非交互（release type / 版本号直选），
  // 缺省或 "prompt" 走交互菜单；preid / currentVersion 缺它则 pre* 释放算错
  // 标识、新版本基线仍来自文件探测
  let release = config_str(config.release.as_deref(), "release")?;
  let preid = config_str(config.preid.as_deref(), "preid")?;
  let current_version = config_str(config.current_version.as_deref(), "currentVersion")?;

  // ---- confirm 门控（COL-60）：交互选定版本即确认，不再二次问（文档语义：
  // 「命令行交互选定版本后不会再问」）；非交互路径按 merged confirm 执行——
  // 缺省 true 二次确认（上游语义），CI / 脚本场景配 confirm = false ----
  let interactive = release.as_deref().is_none_or(|r| r == "prompt");
  if interactive {
    config.confirm = Some(false);
  }

  // ---- 最近 tag（真实 tag 名；无 tag / 非 git 仓库软失败 None）----
  let current_tag = crate::git::get_last_git_tag(cwd)?;

  // ---- 版本确定（JS：`versionBumpInfo()`）----
  // 上游 parity：operation.files = options.files（收集前清单；含未命中的
  // 显式路径）——当前版本的来源探测随它（显式点名文件参与来源）；文件更新
  // 判定清单在 bump 段另经 normalize_files 收集（上游同名量两者分开）
  let info_files = config.files.clone().unwrap_or_default();
  let state = crate::info::version_bump_info(
    &crate::info::BumpInfoOptions {
      release: release.as_deref(),
      files: &info_files,
      current_version: current_version.as_deref(),
      preid: preid.as_deref(),
    },
    cwd,
  )?;

  // ---- changelog：存在 tag 才生成（spinner → 进度打印）----
  let changelog = match current_tag {
    Some(tag) => {
      let outcome = crate::changelog::generate_changelog_with(
        eff,
        &crate::changelog::GenerateChangelogOptions {
          overrides: options.overrides.clone(),
          from: tag,
          to: state.new_version.clone(),
        },
        cwd,
      )?;
      display(&format!(
        "{} Update {} success",
        dialoguer::console::style("✔").green(),
        outcome.output
      ));
      Some(outcome)
    }
    None => None,
  };

  // ---- versionBump（config 形状产物 + release 固定为已确定的新版本；
  // 显示汇透传——其进度行与编排成功行汇入同一计划）----
  let bump_options = BumpOptions::from_config(&config, &state.new_version);
  let bump = crate::bump::version_bump_at(eff, &bump_options, cwd, display, &mut |_| {})?;

  // ---- 平台 Release（spinner → 进度打印）----
  if let Some(provider) = options.provider {
    let markdown = changelog
      .as_ref()
      .map(|c| c.markdown.as_str())
      .unwrap_or("");
    crate::release::create_release_with(
      eff,
      provider,
      &state.new_version,
      markdown,
      cwd,
      options.overrides.as_ref(),
    )?;
    display(&format!(
      "{} [{}] add release v{} success",
      dialoguer::console::style("✔").green(),
      provider.display(),
      state.new_version
    ));
  }

  Ok(BumpVersionOutcome {
    state,
    changelog,
    bump,
    config,
  })
}

/// 字符串型配置键提取（COL-60）：缺省 / null 为 None（结构体已归一为
/// None）；空串报错——配置写错不允许静默回落（release 空串若静默当缺省
/// 会意外弹交互菜单；空串报错对齐上游 `release: ""` 经 loose 解析抛错的
/// 行为）；非字符串类型由形状解析在前报错，不到此处
fn config_str(value: Option<&str>, key: &str) -> Result<Option<String>, OrchestrateError> {
  match value {
    None => Ok(None),
    Some(s) if !s.is_empty() => Ok(Some(s.to_owned())),
    Some(_) => Err(OrchestrateError::Config {
      message: format!(
        "config key \"{key}\" must be a non-empty string — fix the value or remove the key"
      ),
    }),
  }
}
