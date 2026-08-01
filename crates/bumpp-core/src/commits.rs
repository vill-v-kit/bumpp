//! conventional 提交解析与获取：对齐上游 tiny-conventional-commits-parser 正则与
//! bumpp v11 的 `getRecentCommits` / `determineSemverChange`。

use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::version::ReleaseType;

/// 上游 `ConventionalCommitRegex`：未锚定、`/i`（type 保留原样大小写）、
/// 可选 emoji 前缀（gitmoji `:code:` 或 unicode emoji 区间）
static CONVENTIONAL_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"(?i)(?::.+:|[\u{1F300}-\u{1F3FF}]|[\u{1F400}-\u{1F64F}]|[\u{1F680}-\u{1F6FF}]|[\u{2600}-\u{2B55}])? *([a-z]+)(\((.+)\))?(!)?: (.+)",
  )
  .unwrap()
});

/// 上游 `BreakingRE`（body 中的 BREAKING CHANGE / BREAKING-CHANGE / breaking changes）
static BREAKING_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?i)breaking[ -]changes?:").unwrap());

/// 上游 `GIT_LOG_FORMAT`
const GIT_LOG_FORMAT: &str = "%h|%s|%an|%ae|%ad|%b[GIT_LOG_COMMIT_END]";
const COMMIT_END: &str = "[GIT_LOG_COMMIT_END]";

/// 一条提交的解析结果（上游 parseCommit 的版本推断相关部分；
/// authors / references 属展示层，暂不移植）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
  pub short_hash: String,
  pub message: String,
  pub commit_type: String,
  pub scope: String,
  pub description: String,
  pub is_breaking: bool,
  pub is_conventional: bool,
}

/// 上游 `parseCommit`：`!` 标记或 body 含 BREAKING CHANGE 即 breaking
pub fn parse_commit(short_hash: &str, message: &str, body: &str) -> CommitInfo {
  let m = CONVENTIONAL_RE.captures(message);
  let group = |n: usize| {
    m.as_ref()
      .and_then(|c| c.get(n))
      .map(|g| g.as_str().to_owned())
  };
  let marker_breaking = m.as_ref().is_some_and(|c| c.get(4).is_some());
  CommitInfo {
    short_hash: short_hash.to_owned(),
    message: message.to_owned(),
    commit_type: group(1).unwrap_or_default(),
    scope: group(3).unwrap_or_default(),
    description: group(5).unwrap_or_else(|| message.to_owned()),
    is_breaking: marker_breaking || BREAKING_RE.is_match(body),
    is_conventional: m.is_some(),
  }
}

/// 上游 `determineSemverChange`：存在 breaking → major，存在 feat（区分大小写）→ minor，否则 patch
pub fn determine_semver_change(commits: &[CommitInfo]) -> ReleaseType {
  let (mut has_major, mut has_minor) = (false, false);
  for commit in commits {
    if commit.is_breaking {
      has_major = true;
    } else if commit.commit_type == "feat" {
      has_minor = true;
    }
  }
  if has_major {
    ReleaseType::Major
  } else if has_minor {
    ReleaseType::Minor
  } else {
    ReleaseType::Patch
  }
}

/// 静默执行 git 命令（查询类）；上游 execCommand 吞掉一切错误 → None
fn git_output(cwd: &Path, args: &[&str]) -> Option<std::process::Output> {
  let output = Command::new("git")
    .args(args)
    .current_dir(cwd)
    .output()
    .ok()?;
  output.status.success().then_some(output)
}

/// 上游 `getLastGitTag`：`git describe --tags --abbrev=0`，失败/无 tag → None
fn get_last_git_tag(cwd: &Path) -> Option<String> {
  let output = git_output(cwd, &["describe", "--tags", "--abbrev=0"])?;
  let tag = String::from_utf8_lossy(&output.stdout).trim().to_owned();
  (!tag.is_empty()).then_some(tag)
}

/// 上游 `getRecentCommits`：自上次 tag（或 from）以来的提交；
/// 上游 execCommand 吞掉一切错误（非 git 仓库、零提交仓库等）→ 返回空列表
pub fn get_recent_commits(cwd: &Path, from: Option<&str>, to: Option<&str>) -> Vec<CommitInfo> {
  let from = from.map(str::to_owned).or_else(|| get_last_git_tag(cwd));
  let to = to.unwrap_or("HEAD");
  let range = match &from {
    Some(f) => format!("{f}...{to}"),
    None => to.to_string(),
  };
  let Some(output) = git_output(
    cwd,
    &[
      "--no-pager",
      "log",
      &range,
      &format!("--pretty={GIT_LOG_FORMAT}"),
    ],
  ) else {
    return vec![];
  };
  let text = String::from_utf8_lossy(&output.stdout);
  let text = text.trim();
  if text.is_empty() {
    return vec![];
  }
  text
    .split(&format!("{COMMIT_END}\n"))
    .filter(|chunk| !chunk.is_empty())
    // 注：上游 split 后最后一个提交的 body 会残留标记文本（对 breaking 检测惰性无害）；
    // 此处剥离，行为等价但更干净
    .map(|chunk| parse_raw_commit(chunk.trim_end_matches(COMMIT_END)))
    .collect()
}

/// 上游 `parseRawCommit`：`|` 分段，body 段以 \n 重组（filter(Boolean)：丢弃空段）
fn parse_raw_commit(chunk: &str) -> CommitInfo {
  let mut parts = chunk.split('|');
  let short_hash = parts.next().unwrap_or("");
  let message = parts.next().unwrap_or("");
  let _author_name = parts.next();
  let _author_email = parts.next();
  let _date = parts.next();
  let body = parts
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n");
  parse_commit(short_hash, message, &body)
}
