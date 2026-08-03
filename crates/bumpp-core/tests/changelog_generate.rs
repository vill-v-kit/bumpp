//! generateChangelog 编排（ADR-0012）：端到端流程与 N1/C1/C2/C3 修复回归。

mod common;

use std::fs;

use bumpp_core::changelog::{generate_changelog, GenerateChangelogOptions};
use serde_json::{json, Map, Value};
use tempfile::TempDir;

/// 初始化仓库（默认**不写 package.json**——纯 cargo 形态即 N1 回归面）：
/// main 分支、本地身份、init 提交、打 tag，随后两个 conventional 提交
fn init_repo(dir: &TempDir, tag: &str) -> std::path::PathBuf {
  common::isolate_global_home();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  common::git(&path, &["config", "commit.gpgsign", "false"]);
  fs::write(path.join("f.txt"), "init\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  common::git(&path, &["tag", tag]);
  // repo 缺省自 git remote 解析（changelogen `config.repo ||= resolveRepoConfig` 同位）
  common::git(
    &path,
    &["remote", "add", "origin", "git@github.com:owner/repo.git"],
  );
  fs::write(path.join("a.txt"), "a\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "feat(ui): add x (#12)"]);
  fs::write(path.join("b.txt"), "b\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "fix: repair y"]);
  path
}

fn options(
  from: &str,
  to: &str,
  overrides: Option<Map<String, Value>>,
) -> GenerateChangelogOptions {
  GenerateChangelogOptions {
    overrides,
    from: from.to_owned(),
    to: to.to_owned(),
  }
}

#[test]
fn generate_changelog_end_to_end_writes_inserts_and_commits() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  let result = generate_changelog(&options("v1.0.0", "1.1.0", None), &path).unwrap();

  assert!(
    result.markdown.starts_with("## v1.1.0"),
    "{:?}",
    result.markdown
  );
  // 新文件走追加分支：`# Changelog\n\n` + `\n` + markdown + `\n\n`
  let expected_file = format!("# Changelog\n\n\n{}\n\n", result.markdown);
  let written = fs::read_to_string(path.join("CHANGELOG.md")).unwrap();
  assert_eq!(written, expected_file, "初始内容 + 追加");
  assert_eq!(result.changelog_md, written, "changelogMD 与写盘内容一致");

  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: update CHANGELOG.md",
    "默认 commitMessage（{{output}} 已替换）"
  );
  assert_eq!(
    common::git(
      &path,
      &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]
    ),
    "CHANGELOG.md",
    "N1：提交仅含实际写出的 output 文件"
  );
  assert_eq!(
    common::git(&path, &["status", "--porcelain"]),
    "",
    "工作区干净"
  );
}

#[test]
fn generate_changelog_inserts_before_first_existing_entry() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  fs::write(
    path.join("CHANGELOG.md"),
    "# Changelog\n\n## v1.0.0\n\nold entry\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "docs: seed changelog"]);

  let result = generate_changelog(&options("v1.0.0", "1.1.0", None), &path).unwrap();
  let expected = format!(
    "# Changelog\n\n{}\n\n## v1.0.0\n\nold entry\n",
    result.markdown
  );
  assert_eq!(result.changelog_md, expected, "首个 `^###?` 条目前插入");
}

#[test]
fn generate_changelog_commit_false_only_writes_file() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  // C2：提交行为跟随统一配置中的 bumpp commit 开关
  fs::write(path.join(".vbumpprc.json"), r#"{ "commit": false }"#).unwrap();
  let head_before = common::git(&path, &["rev-parse", "HEAD"]);

  generate_changelog(&options("v1.0.0", "1.1.0", None), &path).unwrap();

  assert!(path.join("CHANGELOG.md").exists(), "文件仍写出");
  assert_eq!(
    common::git(&path, &["rev-parse", "HEAD"]),
    head_before,
    "commit: false 不产生提交"
  );
  assert_eq!(
    common::git(&path, &["status", "--porcelain"]),
    "?? .vbumpprc.json\n?? CHANGELOG.md",
    "不 add 不 commit，两文件未跟踪"
  );
}

#[test]
fn generate_changelog_non_v_prefix_tag_diff_and_compare() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "release-1.0.0");
  // C1：from 为真实 tag 名（非 v 前缀项目不再找不到 ref）
  let result = generate_changelog(&options("release-1.0.0", "1.1.0", None), &path).unwrap();
  assert!(result.markdown.contains("Add x"), "diff 以真实 tag 为界");
  assert!(
    result
      .changelog_md
      .contains("compare/release-1.0.0...v1.1.0"),
    "compare 链接 from 用真实 tag 名：{}",
    result.changelog_md
  );
}

#[test]
fn generate_changelog_commit_string_still_commits() {
  // 上游 commit 为 Either<bool, String>：字符串 = 自定义提交信息形态、视为开启
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  fs::write(
    path.join(".vbumpprc.json"),
    r#"{ "commit": "chore: release v%s" }"#,
  )
  .unwrap();
  let head_before = common::git(&path, &["rev-parse", "HEAD"]);
  generate_changelog(&options("v1.0.0", "1.1.0", None), &path).unwrap();
  assert_ne!(
    common::git(&path, &["rev-parse", "HEAD"]),
    head_before,
    "truthy 字符串不阻断提交"
  );
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: update CHANGELOG.md"
  );
}

#[test]
fn generate_changelog_custom_commit_message() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  // C3：changelog 段 commitMessage 配置位
  fs::write(
    path.join(".vbumpprc.json"),
    r#"{ "changelog": { "commitMessage": "docs: 更新 {{output}}" } }"#,
  )
  .unwrap();
  generate_changelog(&options("v1.0.0", "1.1.0", None), &path).unwrap();
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "docs: 更新 CHANGELOG.md"
  );
}

#[test]
fn generate_changelog_overrides_passthrough() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  // overrides 扁平透传：changelog 键生效于 output 与提交信息
  let overrides = serde_json::from_value(json!({
    "changelog": { "output": "HISTORY.md" }
  }))
  .unwrap();
  let result = generate_changelog(&options("v1.0.0", "1.1.0", Some(overrides)), &path).unwrap();
  assert!(path.join("HISTORY.md").exists());
  assert!(!path.join("CHANGELOG.md").exists());
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: update HISTORY.md"
  );
  assert_eq!(
    common::git(
      &path,
      &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]
    ),
    "HISTORY.md"
  );
  assert!(result.markdown.starts_with("## v1.1.0"));
}

#[test]
fn generate_changelog_bad_from_errors() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir, "v1.0.0");
  let err = generate_changelog(&options("nonexistent-tag", "1.1.0", None), &path).unwrap_err();
  assert!(
    err.to_string().contains("nonexistent-tag") || err.to_string().contains("unknown revision"),
    "错误应含上下文：{err}"
  );
  assert!(!path.join("CHANGELOG.md").exists(), "失败不写文件");
}
