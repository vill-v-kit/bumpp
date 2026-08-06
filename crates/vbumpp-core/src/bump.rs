//! versionBump 全链路编排：normalize → 当前/新版本 → confirm →
//! preversion → updateFiles → install/execute → version → commit → tag → postversion → push。
//! 时序与行为对齐上游 bumpp v11 `versionBump` / `normalizeOptions`。

use std::error::Error;
use std::fmt;
use std::path::{Component, Path};

use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;

use crate::exec::{run, ExecError};
use crate::git::{git_commit, git_push, git_tag, CommitSpec, TagSpec};
use crate::info::{get_current_version, resolve_new_version, BumpState, InfoError};
use crate::plugins::{self, FilesError, InstallError};
use crate::progress::ProgressEvent;
use crate::scripts::run_script;

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

/// 配置声明的脚本命令（ADR-0011）：三个时序槽位各自的 shell 命令串，
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
  /// 配置声明的脚本命令（ADR-0011）；ignore_scripts 为 true 时全部跳过
  pub scripts: Option<Scripts>,
  pub preid: Option<&'a str>,
  pub current_version: Option<&'a str>,
}

impl<'a> BumpOptions<'a> {
  /// 自合并配置（`load_bump_config` 产物）构造（ADR-0014 编排用）：
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

impl From<InstallError> for BumpError {
  fn from(e: InstallError) -> Self {
    Self::Exec {
      message: e.to_string(),
    }
  }
}

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
        message: format!("confirmation prompt failed: {e}"),
      })?;
    if !yes {
      // 上游 process.exit(1)；库实现改为可读错误，由调用方决定退出码
      return Err(BumpError::Cancelled {
        message: "bump canceled by user".to_string(),
      });
    }
  }

  macro_rules! emit {
    ($event:expr, $script:expr) => {{
      let p = Progress {
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
      };
      // 内置打印（ADR-0002）：仿 consola 样式，文件事件取最后一个（本次事件的文件）
      let file = match p.event {
        ProgressEvent::FileUpdated => p.updated_files.last().map(String::as_str),
        ProgressEvent::FileSkipped => p.skipped_files.last().map(String::as_str),
        _ => None,
      };
      crate::progress::print_line(p.event, p.script, p.new_version, file, cwd);
      progress(&p);
    }};
  }

  // ---- preversion → updateFiles → install/execute → version → git → postversion → push ----
  // ADR-0011：脚本来自配置声明（scripts 槽位），经系统 shell 执行；
  // 非零退出即报错传播；ignore_scripts 全部跳过
  macro_rules! script_step {
    ($slot:ident) => {
      if !options.ignore_scripts {
        if let Some(command) = options.scripts.as_ref().and_then(|s| s.$slot.as_deref()) {
          run_script(cwd, command)?;
          emit!(ProgressEvent::Script, Some(command));
        }
      }
    };
  }
  script_step!(preversion);

  let outcome = plugins::update_files(&files, cwd, &state.current_version, &state.new_version)?;
  for (event, path) in outcome.events() {
    if *event == ProgressEvent::FileUpdated {
      state.updated_files.push(path.clone());
    } else {
      state.skipped_files.push(path.clone());
    }
    emit!(*event, None);
  }

  // ---- install（ADR-0008：仅当本次有文件被实际更新时，按生态适配触发） ----
  if options.install && !state.updated_files.is_empty() {
    plugins::run_installs(cwd, &state.updated_files)?;
  }

  if let Some(execute) = options.execute {
    // 上游 tokenizeArgs 后无 shell 执行
    let parts = shell_words::split(execute).map_err(|e| BumpError::Exec {
      message: format!("failed to parse execute command: {e}"),
    })?;
    let (program, args) = parts.split_first().ok_or_else(|| BumpError::Exec {
      message: "execute command is empty".to_string(),
    })?;
    run(program, args, cwd)?;
  }

  script_step!(version);

  if let Some(commit) = &commit {
    // COL-61 兜底：pathspec 提交枚举 updated_files，未跟踪路径（收集层
    // 漏网的 gitignored 残留、gitignored Cargo.lock 定向同步、显式 files
    // 点名的未跟踪文件）会让 git 报错炸掉发版——滤为已跟踪子集，被滤文件
    // 保留磁盘修改并逐条 ⚠ 警告（不静默丢弃）；--all 无 pathspec 不涉，
    // 过滤失败 fail-open（git commit 原始报错透出真实问题）
    let tracked_filter = if commit.all {
      None
    } else {
      crate::git::filter_tracked(cwd, &state.updated_files)
    };
    let updated_files: &[String] = match &tracked_filter {
      Some(t) => {
        for f in state.updated_files.iter().filter(|f| !t.contains(f)) {
          // 存储值保持绝对原生（pathspec 依据），打印走显示路径（ADR-0023）
          println!(
            "{} skipping untracked file in commit (left modified on disk): {}",
            dialoguer::console::style("⚠").yellow(),
            crate::display::path(cwd, Path::new(f))
          );
        }
        t.as_slice()
      }
      None => &state.updated_files,
    };
    let (_, message) = git_commit(
      cwd,
      &CommitSpec {
        updated_files,
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

  script_step!(postversion);

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

/// 上游 normalizeOptions 的文件清单归一：空清单启用默认列表，随后 glob 展开（排序、忽略目录）。
/// 默认列表 = 插件底座链上 manifest basenames 的根级并集（ADR-0009）；recursive 时
/// 升级为 `**/` 整树收集模式（替代上游 `packages/**/package.json` 硬编码）
fn normalize_files(options: &BumpOptions, cwd: &Path) -> Vec<String> {
  let patterns: Vec<String> = if options.files.is_empty() {
    plugins::default_file_patterns(options.recursive)
  } else {
    options.files.clone()
  };

  let mut collected: Vec<String> = vec![];
  let mut explicit: Vec<String> = vec![];
  for pattern in &patterns {
    // glob 模式命中属「收集」（默认清单 / -r / 配置 recursive 展开 / 用户
    // 显式 glob）——gitignore 过滤只作用于它们；字面路径是用户点名，
    // 用户意图优先，不过滤（COL-61 spec 边界）
    let is_glob = pattern.contains(['*', '?', '[']);
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
          let rel = rel.to_string_lossy().replace('\\', "/");
          if is_glob {
            collected.push(rel);
          } else {
            explicit.push(rel);
          }
        }
      }
    }
  }
  collected.sort();
  collected.dedup();

  // COL-61：gitignore 过滤（git 仓库内）——glob 裸 walk 会下钻 gitignored
  // 构建残留（target/package 打包暂存、.next 缓存等），残留清单被撞版本号后
  // commit pathspec 撞未跟踪文件炸掉发版（v6.0.0 实例）；git check-ignore
  // 一次进程批量裁决，非 git 仓库 / 检查失败 fail-open 回落裸 walk（现状）
  if let Some(ignored) = crate::git::check_ignored(cwd, &collected) {
    if !ignored.is_empty() {
      let ignored: std::collections::HashSet<String> = ignored.into_iter().collect();
      collected.retain(|f| !ignored.contains(f));
    }
  }
  collected.extend(explicit);
  collected.sort();
  collected.dedup();
  collected
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
