//! versionBump 全链路编排：normalize → 当前/新版本 → confirm →
//! preversion → updateFiles → install/execute → version → commit → tag → postversion → push。
//! 时序与行为对齐上游 bumpp v11 `versionBump` / `normalizeOptions`。
//! 公共类型与错误在模块入口 `bump.rs`。

use std::collections::HashSet;
use std::path::{Component, Path};

use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;

use super::{BumpError, BumpOptions, BumpResults, CommitInput, Progress, TagInput};
use crate::commits::get_recent_commits;
use crate::display;
use crate::effects::{Effects, RealEffects};
use crate::git::{
  check_ignored, filter_tracked, git_commit_with, git_push_with, git_tag_with, CommitSpec, TagSpec,
};
use crate::info::{get_current_version, resolve_new_version, BumpState};
use crate::plugins;
use crate::progress::ProgressEvent;
use crate::scripts::run_script_with;

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
  version_bump_with(&RealEffects, options, cwd, progress)
}

/// `version_bump` 的效应注入形态：全部副作用（scripts / 文件写盘 / install /
/// execute / git commit / tag / push）经效应边界执行，判定与计算留在本流水线
pub fn version_bump_with(
  eff: &dyn Effects,
  options: &BumpOptions,
  cwd: &Path,
  progress: &mut dyn FnMut(&Progress),
) -> Result<BumpResults, BumpError> {
  // 内置进度打印为显示副作用：回传调用方注入的显示汇
  // （dry-run 以闭包捕获为计划行——预演与真实共用同一份打印格式）
  version_bump_at(eff, options, cwd, &mut |line| println!("{line}"), progress)
}

/// 可测内核：进度行打印经 `display` 汇注入；`version_bump_with` 以 stdout
/// 透传（行为逐字节不变），dry-run 以捕获透传（行进入计划而非直接打印）
pub fn version_bump_at(
  eff: &dyn Effects,
  options: &BumpOptions,
  cwd: &Path,
  display: &mut dyn FnMut(&str),
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
  // 未命中显式路径的 Missing 判定（收集清单丢弃不存在的字面路径，真实
  // 执行零事件；dry-run 需要这条 Missing——在收集处钉定，与判定行序合并）
  let missing: Vec<(String, plugins::FileVerdict)> = options
    .files
    .iter()
    .filter(|f| !f.contains(['*', '?', '[']))
    .filter_map(|f| {
      let abs = plugins::resolve(cwd, f);
      (!abs.exists()).then(|| {
        (
          abs.to_string_lossy().into_owned(),
          plugins::FileVerdict::Missing,
        )
      })
    })
    .collect();

  // ---- 版本确定（上游时序：commits → getCurrentVersion → getNewVersion） ----
  let commits = get_recent_commits(cwd, None, None);
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
      // 内置打印：仿 consola 样式，文件事件取最后一个（本次事件的文件）；
      // 行经注入的显示汇（真实 stdout / dry-run 捕获同一份格式）
      let file = match p.event {
        ProgressEvent::FileUpdated => p.updated_files.last().map(String::as_str),
        ProgressEvent::FileSkipped => p.skipped_files.last().map(String::as_str),
        _ => None,
      };
      display(&crate::progress::format_line(
        p.event,
        p.script,
        p.new_version,
        file,
        cwd,
      ));
      progress(&p);
    }};
  }

  // ---- preversion → updateFiles → install/execute → version → git → postversion → push ----
  // 脚本来自配置声明（scripts 槽位），经系统 shell 执行；
  // 非零退出即报错传播；ignore_scripts 全部跳过
  macro_rules! script_step {
    ($slot:ident) => {
      if !options.ignore_scripts {
        if let Some(command) = options.scripts.as_ref().and_then(|s| s.$slot.as_deref()) {
          run_script_with(eff, cwd, command)?;
          emit!(ProgressEvent::Script, Some(command));
        }
      }
    };
  }
  script_step!(preversion);

  let outcome =
    plugins::update_files_with(eff, &files, cwd, &state.current_version, &state.new_version)?;
  for (event, path) in outcome.events() {
    if *event == ProgressEvent::FileUpdated {
      state.updated_files.push(path.clone());
    } else {
      state.skipped_files.push(path.clone());
    }
    emit!(*event, None);
  }

  // ---- install（仅当本次有文件被实际更新时，按生态适配触发） ----
  if options.install && !state.updated_files.is_empty() {
    plugins::run_installs_with(eff, cwd, &state.updated_files)?;
  }

  if let Some(execute) = options.execute {
    // 上游 tokenizeArgs 后无 shell 执行
    let parts = shell_words::split(execute).map_err(|e| BumpError::Exec {
      message: format!("failed to parse execute command: {e}"),
    })?;
    let (program, args) = parts.split_first().ok_or_else(|| BumpError::Exec {
      message: "execute command is empty".to_string(),
    })?;
    eff.run(program, args, cwd)?;
  }

  script_step!(version);

  if let Some(commit) = &commit {
    // 兜底：pathspec 提交枚举 updated_files，未跟踪路径（收集层
    // 漏网的 gitignored 残留、gitignored Cargo.lock 定向同步、显式 files
    // 点名的未跟踪文件）会让 git 报错炸掉发版——滤为已跟踪子集，被滤文件
    // 保留磁盘修改并逐条 ⚠ 警告（不静默丢弃）；--all 无 pathspec 不涉，
    // 过滤失败 fail-open（git commit 原始报错透出真实问题）
    let tracked_filter = if commit.all {
      None
    } else {
      filter_tracked(cwd, &state.updated_files)
    };
    let updated_files: &[String] = match &tracked_filter {
      Some(t) => {
        for f in state.updated_files.iter().filter(|f| !t.contains(f)) {
          // 存储值保持绝对原生（pathspec 依据），打印走显示路径；
          // 行经注入的显示汇（真实 stdout / dry-run 捕获同一份格式）
          display(&format!(
            "{} skipping untracked file in commit (left modified on disk): {}",
            dialoguer::console::style("⚠").yellow(),
            display::path(cwd, Path::new(f))
          ));
        }
        t.as_slice()
      }
      None => &state.updated_files,
    };
    let (_, message) = git_commit_with(
      eff,
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
    let (_, tag_name) = git_tag_with(
      eff,
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
    let _ = git_push_with(eff, cwd, tag.is_some())?;
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
    // 逐文件三态判定：updateFiles 收集文件的判定 + 未命中显式路径的
    // Missing 补行（收集清单丢弃不存在文件、真实执行事件流零 missing——
    // 该补行仅喂 dry-run 的预演判定渲染）
    verdicts: outcome.verdicts().iter().cloned().chain(missing).collect(),
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
/// 默认列表 = 插件底座链上 manifest basenames 的根级并集；recursive 时
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
    // 用户意图优先，不过滤（spec 边界）
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

  // gitignore 过滤（git 仓库内）——glob 裸 walk 会下钻 gitignored
  // 构建残留（target/package 打包暂存、.next 缓存等），残留清单被撞版本号后
  // commit pathspec 撞未跟踪文件炸掉发版（v6.0.0 实例）；git check-ignore
  // 一次进程批量裁决，非 git 仓库 / 检查失败 fail-open 回落裸 walk（现状）
  if let Some(ignored) = check_ignored(cwd, &collected) {
    if !ignored.is_empty() {
      let ignored: HashSet<String> = ignored.into_iter().collect();
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
