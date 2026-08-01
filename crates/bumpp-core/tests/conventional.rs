//! conventional 提交解析与版本推断——对齐上游 bumpp v11
//! （tiny-conventional-commits-parser 正则 + determineSemverChange + getRecentCommits）。

use std::fs;
use std::path::Path;
use std::process::Command;

use bumpp_core::commits::{determine_semver_change, get_recent_commits, parse_commit};
use bumpp_core::version::{next_version, next_versions, ReleaseType};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
  let status = Command::new("git")
    .args(args)
    .current_dir(dir)
    .output()
    .unwrap();
  assert!(status.status.success(), "git {args:?} 失败");
}

fn init_repo(dir: &TempDir) -> std::path::PathBuf {
  let path = dir.path().to_path_buf();
  git(&path, &["init", "-b", "main"]);
  git(&path, &["config", "user.email", "test@example.com"]);
  git(&path, &["config", "user.name", "Test"]);
  path
}

fn commit(dir: &Path, message: &str) {
  let seq = fs::read_dir(dir)
    .unwrap()
    .filter(|e| {
      e.as_ref()
        .is_ok_and(|e| e.file_name().to_string_lossy().starts_with("commit-"))
    })
    .count();
  fs::write(dir.join(format!("commit-{seq:03}")), message).unwrap();
  git(dir, &["add", "."]);
  git(dir, &["commit", "-m", message]);
}

// ---- parse_commit：tiny-conventional-commits-parser 正则 parity ----

#[test]
fn parse_basic_conventional_commit() {
  let c = parse_commit("abc123", "feat: add x", "");
  assert!(c.is_conventional);
  assert_eq!(c.commit_type, "feat");
  assert_eq!(c.scope, "");
  assert_eq!(c.description, "add x");
  assert!(!c.is_breaking);
}

#[test]
fn parse_scope_and_breaking_marker() {
  let c = parse_commit("abc123", "fix(core)!: patch bug", "");
  assert_eq!(c.commit_type, "fix");
  assert_eq!(c.scope, "core");
  assert!(c.is_breaking);
}

#[test]
fn parse_breaking_body_variants() {
  for body in [
    "BREAKING CHANGE: removed",
    "BREAKING-CHANGE: removed",
    "breaking changes: gone",
  ] {
    let c = parse_commit("abc123", "feat: add x", body);
    assert!(c.is_breaking, "body={body:?} 应判定 breaking");
  }
}

#[test]
fn parse_type_preserves_raw_case() {
  // 上游 /i 匹配但保留原样大小写；determineSemverChange 的 === 'feat' 区分大小写
  let c = parse_commit("abc123", "FEAT: add x", "");
  assert!(c.is_conventional);
  assert_eq!(c.commit_type, "FEAT");
}

#[test]
fn parse_emoji_prefixes() {
  assert_eq!(
    parse_commit("h", ":sparkles: feat: x", "").commit_type,
    "feat"
  );
  assert_eq!(parse_commit("h", "✨ feat: x", "").commit_type, "feat");
}

#[test]
fn parse_non_conventional_message() {
  let c = parse_commit("h", "just some words", "");
  assert!(!c.is_conventional);
  assert_eq!(c.commit_type, "");
  assert_eq!(c.description, "just some words");
}

#[test]
fn parse_unanchored_like_upstream() {
  // 上游正则未锚定：消息中段出现 type: 也算 conventional（parity 怪癖，如实复刻）
  let c = parse_commit("h", "random words feat: something", "");
  assert_eq!(c.commit_type, "feat");
}

// ---- determine_semver_change ----

#[test]
fn determine_semver_change_matrix() {
  let commits = |msgs: &[&str]| {
    msgs
      .iter()
      .map(|m| parse_commit("h", m, ""))
      .collect::<Vec<_>>()
  };
  assert_eq!(determine_semver_change(&commits(&[])), ReleaseType::Patch);
  assert_eq!(
    determine_semver_change(&commits(&["fix: a"])),
    ReleaseType::Patch
  );
  assert_eq!(
    determine_semver_change(&commits(&["feat: a"])),
    ReleaseType::Minor
  );
  assert_eq!(
    determine_semver_change(&commits(&["feat!: a"])),
    ReleaseType::Major
  );
  assert_eq!(
    determine_semver_change(&commits(&["feat: a", "feat!: b"])),
    ReleaseType::Major
  );
  // 大写 FEAT 不等于 feat（上游 === 比较区分大小写）
  assert_eq!(
    determine_semver_change(&commits(&["FEAT: a"])),
    ReleaseType::Patch
  );
}

// ---- next_version / next_versions 接入 conventional ----

#[test]
fn conventional_release_with_commits() {
  let commits = |msgs: &[&str]| {
    msgs
      .iter()
      .map(|m| parse_commit("h", m, ""))
      .collect::<Vec<_>>()
  };
  assert_eq!(
    next_version(
      "1.2.3",
      ReleaseType::Conventional,
      None,
      &commits(&["feat!: a"])
    )
    .unwrap(),
    "2.0.0"
  );
  assert_eq!(
    next_version(
      "1.2.3",
      ReleaseType::Conventional,
      None,
      &commits(&["feat: a"])
    )
    .unwrap(),
    "1.3.0"
  );
  assert_eq!(
    next_version(
      "1.2.3",
      ReleaseType::Conventional,
      None,
      &commits(&["fix: a"])
    )
    .unwrap(),
    "1.2.4"
  );
  // 预发行版本：conventional 一律解析为 prerelease（不看提交）
  assert_eq!(
    next_version(
      "1.0.0-beta.0",
      ReleaseType::Conventional,
      None,
      &commits(&["feat!: a"])
    )
    .unwrap(),
    "1.0.0-beta.1"
  );
  // 0→1 修正只看请求类型：conventional 不修正
  assert_eq!(
    next_version(
      "1.0.0-0",
      ReleaseType::Conventional,
      Some("preid"),
      &commits(&["feat: a"])
    )
    .unwrap(),
    "1.0.0-preid.0"
  );
}

#[test]
fn next_versions_includes_conventional() {
  let commits = [parse_commit("h", "feat: a", "")];
  let next = next_versions("1.2.3", None, &commits).unwrap();
  assert_eq!(next.conventional, "1.3.0");
  let next = next_versions("1.2.3", None, &[]).unwrap();
  assert_eq!(next.conventional, "1.2.4");
}

// ---- get_recent_commits（真实 git 仓库） ----

#[test]
fn recent_commits_since_last_tag() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  commit(&path, "chore: init");
  git(&path, &["tag", "v1.0.0"]);
  commit(&path, "feat: new thing");
  commit(&path, "fix!: broke it");
  let commits = get_recent_commits(&path, None, None);
  assert_eq!(commits.len(), 2, "只含 tag 之后的提交");
  assert_eq!(commits[0].commit_type, "fix");
  assert!(commits[0].is_breaking);
  assert_eq!(commits[1].commit_type, "feat");
}

#[test]
fn recent_commits_without_tag_returns_all() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  commit(&path, "chore: init");
  commit(&path, "feat: new thing");
  let commits = get_recent_commits(&path, None, None);
  assert_eq!(commits.len(), 2);
}

#[test]
fn recent_commits_outside_repo_returns_empty() {
  let dir = TempDir::new().unwrap();
  // 上游 execCommand 吞错：非 git 仓库 → 空提交列表
  assert!(get_recent_commits(dir.path(), None, None).is_empty());
}

#[test]
fn recent_commits_in_zero_commit_repo_returns_empty() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  // 零提交仓库：describe 失败 → git log HEAD 也失败 → 空列表（上游吞错 parity）
  assert!(get_recent_commits(&path, None, None).is_empty());
}
