//! git 操作（commit / tag / push）——在真实临时 git 仓库中验证，对齐上游 bumpp v11 行为。

use std::fs;
use std::path::Path;
use std::process::Command;

use bumpp_core::git::{format_version_string, git_commit, git_push, git_tag, CommitSpec, TagSpec};
use bumpp_core::progress::ProgressEvent;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) -> String {
  let output = Command::new("git")
    .args(args)
    .current_dir(dir)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "git {args:?} 失败：{}",
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// 初始化仓库：main 分支、本地身份、关闭签名、一个初始提交
fn init_repo(dir: &TempDir) -> std::path::PathBuf {
  let path = dir.path().to_path_buf();
  git(&path, &["init", "-b", "main"]);
  git(&path, &["config", "user.email", "test@example.com"]);
  git(&path, &["config", "user.name", "Test"]);
  git(&path, &["config", "commit.gpgsign", "false"]);
  git(&path, &["config", "tag.gpgsign", "false"]);
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "init"]);
  path
}

fn commit_spec<'a>(updated_files: &'a [String], message: &'a str) -> CommitSpec<'a> {
  CommitSpec {
    updated_files,
    all: false,
    no_verify: false,
    sign: false,
    message,
    new_version: "2.0.0",
  }
}

#[test]
fn format_version_string_replaces_or_appends() {
  assert_eq!(
    format_version_string("release v%s", "2.0.0"),
    "release v2.0.0"
  );
  assert_eq!(
    format_version_string("chore: release v", "2.0.0"),
    "chore: release v2.0.0"
  );
  assert_eq!(format_version_string("v", "2.0.0"), "v2.0.0");
  assert_eq!(
    format_version_string("%s and %s", "2.0.0"),
    "2.0.0 and 2.0.0"
  );
}

#[test]
fn commit_commits_only_updated_files_with_formatted_message() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("a.txt"), "a").unwrap();
  fs::write(path.join("b.txt"), "b").unwrap();
  git(&path, &["add", "."]);
  let files = vec!["a.txt".to_string()];
  let (event, message) = git_commit(&path, &commit_spec(&files, "chore: release v%s")).unwrap();
  assert_eq!(event, ProgressEvent::GitCommit);
  assert_eq!(message, "chore: release v2.0.0");
  assert_eq!(
    git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: release v2.0.0"
  );
  // 只提交 updated_files 列出的文件，b.txt 仍在暂存区
  let status = git(&path, &["status", "--porcelain"]);
  assert!(status.contains("b.txt"), "b.txt 不应被提交：{status}");
}

#[test]
fn commit_allow_empty() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let files: Vec<String> = vec![];
  git_commit(&path, &commit_spec(&files, "release v%s")).unwrap();
  assert_eq!(git(&path, &["log", "-1", "--pretty=%s"]), "release v2.0.0");
}

#[test]
fn commit_all_includes_everything() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("a.txt"), "a").unwrap();
  fs::write(path.join("b.txt"), "b").unwrap();
  git(&path, &["add", "."]); // --all 只覆盖已跟踪文件，先入索引
  let spec = CommitSpec {
    all: true,
    ..commit_spec(&[], "release v%s")
  };
  git_commit(&path, &spec).unwrap();
  assert_eq!(git(&path, &["status", "--porcelain"]), "", "工作区应干净");
}

#[test]
fn commit_no_verify_skips_hooks() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let hook = path.join(".git/hooks/pre-commit");
  fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
  }
  // noVerify=false → hook 拒绝，commit 失败
  let err = git_commit(&path, &commit_spec(&[], "release v%s")).unwrap_err();
  assert!(
    err.to_string().contains("git commit"),
    "错误应含命令：{err}"
  );
  // noVerify=true → 跳过 hook
  let spec = CommitSpec {
    no_verify: true,
    ..commit_spec(&[], "release v%s")
  };
  git_commit(&path, &spec).unwrap();
  assert_eq!(git(&path, &["log", "-1", "--pretty=%s"]), "release v2.0.0");
}

#[test]
fn tag_creates_annotated_tag_with_formatted_name_and_message() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let (event, tag_name) = git_tag(
    &path,
    &TagSpec {
      name: "v%s",
      message: "chore: release v%s",
      sign: false,
      new_version: "2.0.0",
    },
  )
  .unwrap();
  assert_eq!(event, ProgressEvent::GitTag);
  assert_eq!(tag_name, "v2.0.0");
  assert_eq!(git(&path, &["tag", "-l"]), "v2.0.0");
  let body = git(
    &path,
    &["for-each-ref", "refs/tags/v2.0.0", "--format=%(contents)"],
  );
  assert!(body.contains("chore: release v2.0.0"), "附注信息：{body}");
}

#[test]
fn tag_without_placeholder_appends_version() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let (_, tag_name) = git_tag(
    &path,
    &TagSpec {
      name: "release-",
      message: "chore: release v",
      sign: false,
      new_version: "2.0.0",
    },
  )
  .unwrap();
  assert_eq!(tag_name, "release-2.0.0");
  let body = git(
    &path,
    &[
      "for-each-ref",
      "refs/tags/release-2.0.0",
      "--format=%(contents)",
    ],
  );
  assert!(body.contains("chore: release v2.0.0"), "附注信息：{body}");
}

#[test]
fn push_pushes_commits_and_tags_to_remote() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let bare = TempDir::new().unwrap();
  git(&path, &["init", "--bare", bare.path().to_str().unwrap()]);
  git(
    &path,
    &["remote", "add", "origin", bare.path().to_str().unwrap()],
  );
  git(&path, &["push", "-u", "origin", "main"]); // 建立上游跟踪（对齐真实使用场景）
  fs::write(path.join("a.txt"), "a").unwrap();
  git(&path, &["add", "."]);
  let files = vec!["a.txt".to_string()];
  git_commit(&path, &commit_spec(&files, "release v%s")).unwrap();
  git_tag(
    &path,
    &TagSpec {
      name: "v%s",
      message: "release v%s",
      sign: false,
      new_version: "2.0.0",
    },
  )
  .unwrap();
  let event = git_push(&path, true).unwrap();
  assert_eq!(event, ProgressEvent::GitPush);
  // 远端收到提交与 tag
  let remote_log = git(bare.path(), &["log", "-1", "--pretty=%s", "main"]);
  assert_eq!(remote_log, "release v2.0.0");
  let remote_tags = git(bare.path(), &["tag", "-l"]);
  assert_eq!(remote_tags, "v2.0.0");
}

#[test]
fn push_without_tags_flag_skips_tag_push() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let bare = TempDir::new().unwrap();
  git(&path, &["init", "--bare", bare.path().to_str().unwrap()]);
  git(
    &path,
    &["remote", "add", "origin", bare.path().to_str().unwrap()],
  );
  git(&path, &["push", "-u", "origin", "main"]);
  git(&path, &["tag", "v9.9.9"]);
  git_push(&path, false).unwrap();
  assert_eq!(git(bare.path(), &["tag", "-l"]), "", "不应推送 tag");
}

#[test]
fn git_failure_error_includes_stderr() {
  let dir = TempDir::new().unwrap();
  // 非 git 仓库中 commit → 错误含 stderr
  let err = git_commit(dir.path(), &commit_spec(&[], "release v%s")).unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("not a git repository"),
    "错误应含 stderr：{msg}"
  );
}
