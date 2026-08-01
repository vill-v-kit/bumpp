//! versionBump 全链路编排：normalize → 当前/新版本 → confirm →
//! preversion → updateFiles → install/execute → version → commit → tag → postversion → push。
//! 时序与行为对齐上游 bumpp v11 `versionBump` / `normalizeOptions`。

use std::error::Error;
use std::fmt;
use std::path::{Component, Path};

use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;

use crate::exec::{run, ExecError};
use crate::files::{self, FilesError};
use crate::git::{git_commit, git_push, git_tag, CommitSpec, TagSpec};
use crate::info::{get_current_version, resolve_new_version, BumpState, InfoError};
use crate::progress::ProgressEvent;
use crate::scripts::run_npm_script;

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
  pub preid: Option<&'a str>,
  pub current_version: Option<&'a str>,
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

/// 上游 `operation.results`
#[derive(Debug, PartialEq, Eq)]
pub struct BumpResults {
  pub release: Option<String>,
  pub current_version: String,
  pub new_version: String,
  pub commit: Option<String>,
  pub tag: Option<String>,
  pub updated_files: Vec<String>,
  pub skipped_files: Vec<String>,
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

/// 上游默认文件清单（files 为空时）
const DEFAULT_FILES: [&str; 6] = [
  "package.json",
  "package-lock.json",
  "jsr.json",
  "jsr.jsonc",
  "deno.json",
  "deno.jsonc",
];

/// 上游 glob 忽略目录（`**/{...}/**`）
const IGNORED_DIRS: [&str; 6] = [
  ".git",
  "node_modules",
  "bower_components",
  "__tests__",
  "fixtures",
  "fixture",
];

/// 上游 `versionBump` 全链路
pub fn version_bump(
  options: &BumpOptions,
  cwd: &Path,
  progress: &mut dyn FnMut(&Progress),
) -> Result<BumpResults, BumpError> {
  // ---- normalizeOptions ----
  let preid = options.preid.unwrap_or("beta");
  let tag = match &options.tag {
    Some(TagInput::Name(name)) => Some(NormalizedTag { name }),
    Some(TagInput::Bool(true)) => Some(NormalizedTag { name: "v" }),
    _ => None,
  };
  // 上游：tag 或 push 开启时 commit 对象强制存在（tag 需要承载提交）
  let commit = match &options.commit {
    Some(CommitInput::Message(message)) => Some(NormalizedCommit {
      all: options.all,
      no_verify: options.no_verify,
      message,
    }),
    Some(CommitInput::Bool(true)) => Some(NormalizedCommit {
      all: options.all,
      no_verify: options.no_verify,
      message: "chore: release v",
    }),
    _ if tag.is_some() || options.push => Some(NormalizedCommit {
      all: options.all,
      no_verify: options.no_verify,
      message: "chore: release v",
    }),
    _ => None,
  };
  let files = normalize_files(options, cwd);

  // ---- 版本确定（上游时序：commits → getCurrentVersion → getNewVersion） ----
  let commits = crate::commits::get_recent_commits(cwd, None, None);
  let (current_version, source) = get_current_version(&files, options.current_version, cwd)?;
  let (release, new_version) =
    resolve_new_version(options.release, Some(preid), &current_version, &commits)?;

  let mut state = BumpState {
    release,
    current_version,
    current_version_source: source,
    new_version,
    ..BumpState::new(String::new(), String::new())
  };

  // ---- confirm（上游 printSummary + confirm prompt；拒绝即取消） ----
  if options.confirm {
    print_summary(&state, &commit, &tag, options);
    let yes = Confirm::with_theme(&ColorfulTheme::default())
      .with_prompt("Bump?")
      .default(true)
      .interact()
      .map_err(|e| BumpError::Cancelled {
        message: format!("确认交互失败：{e}"),
      })?;
    if !yes {
      // 上游 process.exit(1)；库实现改为可读错误，由调用方决定退出码
      return Err(BumpError::Cancelled {
        message: "用户取消了本次 bump".to_string(),
      });
    }
  }

  macro_rules! emit {
    ($event:expr, $script:expr) => {
      progress(&Progress {
        event: $event,
        script: $script,
        release: state.release.as_deref(),
        current_version: &state.current_version,
        new_version: &state.new_version,
        // 上游：commit 启用时负载为 state.commitMessage（commit 前为空串）
        commit: if commit.is_some() {
          Some(state.commit_message.as_str())
        } else {
          None
        },
        // 上游：tag 启用时负载为 state.tagName（GitTag 前为空串）
        tag: if tag.is_some() {
          Some(state.tag_name.as_str())
        } else {
          None
        },
        updated_files: &state.updated_files,
        skipped_files: &state.skipped_files,
      })
    };
  }

  // ---- preversion → updateFiles → install/execute → version → git → postversion → push ----
  macro_rules! script_step {
    ($name:literal) => {
      if let Some((_, script)) = run_npm_script(cwd, $name, options.ignore_scripts)? {
        emit!(ProgressEvent::NpmScript, Some(script.as_str()));
      }
    };
  }
  script_step!("preversion");

  let outcome = files::update_files(&files, cwd, &state.current_version, &state.new_version)?;
  for (event, path) in outcome.events() {
    if *event == ProgressEvent::FileUpdated {
      state.updated_files.push(path.clone());
    } else {
      state.skipped_files.push(path.clone());
    }
    emit!(*event, None);
  }

  if options.install {
    let pm = detect_package_manager(cwd)?;
    run(pm, &["install".to_string()], cwd)?;
  }

  if let Some(execute) = options.execute {
    // 上游 tokenizeArgs 后无 shell 执行
    let parts = shell_words::split(execute).map_err(|e| BumpError::Exec {
      message: format!("解析 execute 命令失败：{e}"),
    })?;
    let (program, args) = parts.split_first().ok_or_else(|| BumpError::Exec {
      message: "execute 命令为空".to_string(),
    })?;
    run(program, args, cwd)?;
  }

  script_step!("version");

  if let Some(commit) = &commit {
    let (_, message) = git_commit(
      cwd,
      &CommitSpec {
        updated_files: &state.updated_files,
        all: commit.all,
        no_verify: commit.no_verify,
        sign: options.sign,
        message: commit.message,
        new_version: &state.new_version,
      },
    )?;
    state.commit_message = message;
    emit!(ProgressEvent::GitCommit, None);
  }

  if let Some(tag) = &tag {
    let (_, tag_name) = git_tag(
      cwd,
      &TagSpec {
        name: tag.name,
        // 上游：tag 附注信息复用 commit.message
        message: commit
          .as_ref()
          .map(|c| c.message)
          .unwrap_or("chore: release v"),
        sign: options.sign,
        new_version: &state.new_version,
      },
    )?;
    state.tag_name = tag_name;
    emit!(ProgressEvent::GitTag, None);
  }

  script_step!("postversion");

  if options.push {
    let _ = git_push(cwd, tag.is_some())?;
    emit!(ProgressEvent::GitPush, None);
  }

  Ok(BumpResults {
    release: state.release,
    current_version: state.current_version,
    new_version: state.new_version,
    commit: if commit.is_some() {
      Some(state.commit_message)
    } else {
      None
    },
    tag: if tag.is_some() {
      Some(state.tag_name)
    } else {
      None
    },
    updated_files: state.updated_files,
    skipped_files: state.skipped_files,
  })
}

struct NormalizedCommit<'a> {
  all: bool,
  no_verify: bool,
  message: &'a str,
}

struct NormalizedTag<'a> {
  name: &'a str,
}

/// 上游 normalizeOptions 的文件清单归一：空清单启用默认列表，随后 glob 展开（排序、忽略目录）
fn normalize_files(options: &BumpOptions, cwd: &Path) -> Vec<String> {
  let patterns: Vec<String> = if options.files.is_empty() {
    let mut defaults: Vec<String> = DEFAULT_FILES.iter().map(|s| s.to_string()).collect();
    if options.recursive {
      // 上游 recursive 默认含 packages/**/package.json（workspace 清单展开由消费侧负责）
      defaults.push("packages/**/package.json".to_string());
    }
    defaults
  } else {
    options.files.clone()
  };

  let mut matched: Vec<String> = vec![];
  for pattern in &patterns {
    let full = cwd.join(pattern);
    let Some(full) = full.to_str() else { continue };
    for entry in glob::glob_with(full, glob::MatchOptions::default())
      .into_iter()
      .flatten()
      .flatten()
    {
      if entry.components().any(|c| {
        matches!(c, Component::Normal(seg) if IGNORED_DIRS.contains(&seg.to_string_lossy().as_ref()))
      }) {
        continue;
      }
      if entry.is_file() {
        if let Ok(rel) = entry.strip_prefix(cwd) {
          matched.push(rel.to_string_lossy().replace('\\', "/"));
        }
      }
    }
  }
  matched.sort();
  matched.dedup();
  matched
}

/// 上游 package-manager-detector 的常用路径：packageManager 字段 → lockfile
fn detect_package_manager(cwd: &Path) -> Result<&'static str, BumpError> {
  if let Ok(text) = std::fs::read_to_string(cwd.join("package.json")) {
    if let Some(jsonc_parser::ast::Value::Object(root)) = crate::jsonc::parse(&text) {
      if let Some(pm) = crate::jsonc::get_prop(&root, "packageManager")
        .and_then(|p| p.value.as_string_lit().cloned())
        .and_then(|s| s.value.split('@').next().map(str::to_owned))
      {
        return Ok(match pm.as_str() {
          "pnpm" => "pnpm",
          "yarn" => "yarn",
          "bun" => "bun",
          _ => "npm",
        });
      }
    }
  }
  for (file, pm) in [
    ("pnpm-lock.yaml", "pnpm"),
    ("package-lock.json", "npm"),
    ("yarn.lock", "yarn"),
    ("bun.lockb", "bun"),
    ("bun.lock", "bun"),
    ("deno.lock", "deno"),
  ] {
    if cwd.join(file).exists() {
      return Ok(pm);
    }
  }
  Err(BumpError::Exec {
    message: "Could not detect package manager, failed to run npm install".to_string(),
  })
}

/// 上游 `printSummary`（confirm 前的变更摘要）
fn print_summary(
  state: &BumpState,
  commit: &Option<NormalizedCommit>,
  tag: &Option<NormalizedTag>,
  options: &BumpOptions,
) {
  use crate::git::format_version_string;
  println!();
  if !options.files.is_empty() {
    for f in &options.files {
      println!("   files {f}");
    }
  }
  if let Some(c) = commit {
    println!(
      "  commit {}",
      format_version_string(c.message, &state.new_version)
    );
  }
  if let Some(t) = tag {
    println!(
      "     tag {}",
      format_version_string(t.name, &state.new_version)
    );
  }
  if let Some(e) = options.execute {
    println!(" execute {e}");
  }
  if options.push {
    println!("    push yes");
  }
  if options.install {
    println!(" install yes");
  }
  println!();
  println!("    from {}", state.current_version);
  println!("      to {}", state.new_version);
  println!();
}
