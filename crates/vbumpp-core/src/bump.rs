//! Bump 域：versionBump 全链路编排的公共类型、错误与入口函数。
//! 主流程在 `flow`（时序对齐上游 bumpp v11 `versionBump` / `normalizeOptions`），
//! dry-run 计划装配在 `plan`（预演与执行同路）。

mod flow;
pub mod plan;

pub use flow::{version_bump, version_bump_at, version_bump_with};

use std::error::Error;
use std::fmt;

use crate::config::BumpConfig;
use crate::exec::ExecError;
use crate::info::InfoError;
use crate::plugins::{FileVerdict, FilesError, InstallError};
use crate::progress::ProgressEvent;

/// commit 选项（上游 `boolean | string`，对象形态上游亦未支持）
#[derive(Debug, Clone, Copy)]
pub enum CommitInput<'a> {
  Bool(bool),
  Message(&'a str),
}

/// tag 选项（上游 `boolean | string`）
#[derive(Debug, Clone, Copy)]
pub enum TagInput<'a> {
  Bool(bool),
  Name(&'a str),
}

/// 配置声明的脚本命令：三个时序槽位各自的 shell 命令串，
/// 经系统 shell 执行；槽位语义与上游 npm scripts 位一致
#[derive(Debug, Clone, Default)]
pub struct Scripts {
  /// updateFiles 之前
  pub preversion: Option<String>,
  /// git commit/tag 之前
  pub version: Option<String>,
  /// git 完成之后、push 之前
  pub postversion: Option<String>,
}

/// versionBump 输入（上游 VersionBumpOptions 的相关子集）
pub struct BumpOptions<'a> {
  /// release type 或版本号；None / "prompt" 走交互 prompt
  pub release: Option<&'a str>,
  /// 文件清单（glob 模式）；为空时启用上游默认 manifest 清单
  pub files: Vec<String>,
  pub recursive: bool,
  pub commit: Option<CommitInput<'a>>,
  pub tag: Option<TagInput<'a>>,
  pub push: bool,
  pub sign: bool,
  pub all: bool,
  pub no_verify: bool,
  pub confirm: bool,
  pub ignore_scripts: bool,
  pub install: bool,
  pub execute: Option<&'a str>,
  /// 配置声明的脚本命令；ignore_scripts 为 true 时全部跳过
  pub scripts: Option<Scripts>,
  pub preid: Option<&'a str>,
  pub current_version: Option<&'a str>,
}

impl<'a> BumpOptions<'a> {
  /// 自合并配置（`load_bump_config` 产物）构造（编排用）：
  /// release 槽固定为交互选定的新版本。bumpp 键无严格 schema（上游 parity），
  /// 类型不符的键按缺失处理回落默认
  pub fn from_merged(
    merged: &'a serde_json::Map<String, serde_json::Value>,
    new_version: &'a str,
  ) -> Self {
    use serde_json::Value;
    let bool_of = |key: &str| merged.get(key).and_then(Value::as_bool).unwrap_or(false);
    let str_of = |key: &str| {
      merged
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    };
    let files = merged
      .get("files")
      .and_then(Value::as_array)
      .map(|a| {
        a.iter()
          .filter_map(Value::as_str)
          .map(str::to_owned)
          .collect()
      })
      .unwrap_or_default();
    let commit = match merged.get("commit") {
      Some(Value::Bool(b)) => Some(CommitInput::Bool(*b)),
      Some(Value::String(s)) if !s.is_empty() => Some(CommitInput::Message(s)),
      _ => None,
    };
    let tag = match merged.get("tag") {
      Some(Value::Bool(b)) => Some(TagInput::Bool(*b)),
      Some(Value::String(s)) if !s.is_empty() => Some(TagInput::Name(s)),
      _ => None,
    };
    let scripts = merged
      .get("scripts")
      .and_then(Value::as_object)
      .map(|s| Scripts {
        preversion: s
          .get("preversion")
          .and_then(Value::as_str)
          .map(str::to_owned),
        version: s.get("version").and_then(Value::as_str).map(str::to_owned),
        postversion: s
          .get("postversion")
          .and_then(Value::as_str)
          .map(str::to_owned),
      });
    Self {
      release: Some(new_version),
      files,
      recursive: bool_of("recursive"),
      commit,
      tag,
      push: bool_of("push"),
      sign: bool_of("sign"),
      all: bool_of("all"),
      no_verify: bool_of("noVerify"),
      confirm: bool_of("confirm"),
      ignore_scripts: bool_of("ignoreScripts"),
      install: bool_of("install"),
      execute: str_of("execute"),
      scripts,
      preid: str_of("preid"),
      current_version: str_of("currentVersion"),
    }
  }

  /// 自形状结构体（`config::shape_of` 产物）构造（编排用）：
  /// release 槽固定为已确定的新版本；逐键语义与 `from_merged` 一致
  /// （execute / preid / currentVersion 与 commit / tag 的空串均按缺失）
  pub fn from_config(config: &'a BumpConfig, new_version: &'a str) -> Self {
    use crate::config::BoolOrString;
    let commit = match &config.commit {
      Some(BoolOrString::Bool(b)) => Some(CommitInput::Bool(*b)),
      Some(BoolOrString::Str(s)) if !s.is_empty() => Some(CommitInput::Message(s)),
      _ => None,
    };
    let tag = match &config.tag {
      Some(BoolOrString::Bool(b)) => Some(TagInput::Bool(*b)),
      Some(BoolOrString::Str(s)) if !s.is_empty() => Some(TagInput::Name(s)),
      _ => None,
    };
    let non_empty = |v: &'a Option<String>| v.as_deref().filter(|s| !s.is_empty());
    Self {
      release: Some(new_version),
      files: config.files.clone().unwrap_or_default(),
      recursive: config.recursive.unwrap_or(false),
      commit,
      tag,
      push: config.push.unwrap_or(false),
      sign: config.sign.unwrap_or(false),
      all: config.all.unwrap_or(false),
      no_verify: config.no_verify.unwrap_or(false),
      confirm: config.confirm.unwrap_or(false),
      ignore_scripts: config.ignore_scripts.unwrap_or(false),
      install: config.install.unwrap_or(false),
      execute: non_empty(&config.execute),
      scripts: config.scripts.as_ref().map(|s| Scripts {
        preversion: s.preversion.clone(),
        version: s.version.clone(),
        postversion: s.postversion.clone(),
      }),
      preid: non_empty(&config.preid),
      current_version: non_empty(&config.current_version),
    }
  }
}

/// 一次进度事件的负载快照（上游 `{ event, script, ...operation.results }`）
#[derive(Debug)]
pub struct Progress<'a> {
  pub event: ProgressEvent,
  pub script: Option<&'a str>,
  pub release: Option<&'a str>,
  pub current_version: &'a str,
  pub new_version: &'a str,
  /// 上游 commit: false（未启用）→ None
  pub commit: Option<&'a str>,
  /// 上游 tag: false（未启用/未执行）→ None
  pub tag: Option<&'a str>,
  pub updated_files: &'a [String],
  pub skipped_files: &'a [String],
}

/// 上游 `operation.results`；`verdicts` 为逐文件三态判定（每个收集文件
/// 恰一条、处理顺序）——dry-run 的预演判定行数据源
#[derive(Debug, PartialEq, Eq)]
pub struct BumpResults {
  pub release: Option<String>,
  pub current_version: String,
  pub new_version: String,
  pub commit: Option<String>,
  pub tag: Option<String>,
  pub updated_files: Vec<String>,
  pub skipped_files: Vec<String>,
  pub verdicts: Vec<(String, FileVerdict)>,
}

#[derive(Debug)]
pub enum BumpError {
  Info { message: String },
  Files { message: String },
  Exec { message: String },
  Cancelled { message: String },
}

impl fmt::Display for BumpError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Info { message }
      | Self::Files { message }
      | Self::Exec { message }
      | Self::Cancelled { message } => f.write_str(message),
    }
  }
}

impl Error for BumpError {}

impl From<InfoError> for BumpError {
  fn from(e: InfoError) -> Self {
    Self::Info {
      message: e.to_string(),
    }
  }
}

impl From<FilesError> for BumpError {
  fn from(e: FilesError) -> Self {
    Self::Files {
      message: e.to_string(),
    }
  }
}

impl From<ExecError> for BumpError {
  fn from(e: ExecError) -> Self {
    Self::Exec {
      message: e.to_string(),
    }
  }
}

impl From<InstallError> for BumpError {
  fn from(e: InstallError) -> Self {
    Self::Exec {
      message: e.to_string(),
    }
  }
}

// 主流程（version_bump* 三入口与归一 / 摘要 helper）见 `flow`；
// dry-run 计划装配见 `plan`。
