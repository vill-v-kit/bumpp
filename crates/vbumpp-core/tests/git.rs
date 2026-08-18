//! git 操作（commit / tag / push）——在真实临时 git 仓库中验证，对齐上游 bumpp v11 行为。

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use vbumpp_core::git::{
  check_ignored, filter_tracked, format_version_string, get_current_git_branch, get_git_diff,
  get_last_git_tag, get_repo_config, git_commit, git_push, git_tag, resolve_repo_config,
  CommitSpec, RepoConfig, TagSpec,
};
use vbumpp_core::progress::ProgressEvent;

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
fn last_git_tag_returns_nearest_reachable_tag() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  git(&path, &["tag", "v0.9.0"]);
  fs::write(path.join("a.txt"), "a").unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "feat: a"]);
  git(&path, &["tag", "v1.0.0"]);
  fs::write(path.join("b.txt"), "b").unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "feat: b"]);
  assert_eq!(
    get_last_git_tag(&path).unwrap(),
    Some("v1.0.0".to_string()),
    "describe --tags 取 HEAD 可达的最近 tag（含轻量 tag）"
  );
}

#[test]
fn last_git_tag_without_tags_returns_none() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  assert_eq!(get_last_git_tag(&path).unwrap(), None, "无 tag 软失败");
}

#[test]
fn last_git_tag_outside_repo_returns_none() {
  let dir = TempDir::new().unwrap();
  assert_eq!(
    get_last_git_tag(dir.path()).unwrap(),
    None,
    "非 git 仓库软失败（对齐 changelogen try/catch → undefined）"
  );
}

#[test]
fn current_git_branch_returns_branch_name() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  assert_eq!(get_current_git_branch(&path).unwrap(), "main");
}

#[test]
fn current_git_branch_outside_repo_errors() {
  let dir = TempDir::new().unwrap();
  assert!(
    get_current_git_branch(dir.path()).is_err(),
    "非 git 仓库报错传播（changelogen 此函数无 catch）"
  );
}

/// 造两个带特征的提交（feat 单行 + fix 含 BREAKING body），返回各自 %h
fn commit_pair(path: &Path) -> (String, String) {
  fs::write(path.join("a.txt"), "a").unwrap();
  git(path, &["add", "."]);
  git(path, &["commit", "-m", "feat: add a"]);
  let feat_hash = git(path, &["log", "-1", "--pretty=%h"]);
  fs::write(path.join("b.txt"), "b").unwrap();
  git(path, &["add", "."]);
  git(
    path,
    &["commit", "-m", "fix: b", "-m", "BREAKING CHANGE: b breaks"],
  );
  let fix_hash = git(path, &["log", "-1", "--pretty=%h"]);
  (feat_hash, fix_hash)
}

#[test]
fn git_diff_returns_commits_in_range_newest_first() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  git(&path, &["tag", "v1.0.0"]);
  let (feat_hash, fix_hash) = commit_pair(&path);
  let commits = get_git_diff(&path, "v1.0.0", None).unwrap();
  assert_eq!(commits.len(), 2, "三点对称差范围：tag 后的两个提交");
  // git log 默认新→旧
  assert_eq!(commits[0].message, "fix: b");
  assert_eq!(commits[0].short_hash, fix_hash);
  assert_eq!(commits[0].author.name, "Test");
  assert_eq!(commits[0].author.email, "test@example.com");
  assert!(
    commits[0].body.contains("BREAKING CHANGE: b breaks"),
    "body 含提交正文：{}",
    commits[0].body
  );
  assert_eq!(commits[1].message, "feat: add a");
  assert_eq!(commits[1].short_hash, feat_hash);
}

#[test]
fn git_diff_with_empty_from_returns_full_history() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  commit_pair(&path);
  let commits = get_git_diff(&path, "", None).unwrap();
  assert_eq!(commits.len(), 3, "from 为空取 HEAD 全史（含 init 提交）");
  assert_eq!(commits[2].message, "init");
}

#[test]
fn git_diff_to_ref_bounds_range() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  git(&path, &["tag", "v1.0.0"]);
  let (feat_hash, _) = commit_pair(&path);
  let commits = get_git_diff(&path, "v1.0.0", Some(&feat_hash)).unwrap();
  assert_eq!(commits.len(), 1, "to 限定上界后只剩 feat 提交");
  assert_eq!(commits[0].message, "feat: add a");
}

fn repo_config(provider: Option<&str>, domain: Option<&str>, repo: Option<&str>) -> RepoConfig {
  RepoConfig {
    provider: provider.map(str::to_owned),
    domain: domain.map(str::to_owned),
    repo: repo.map(str::to_owned),
  }
}

#[test]
fn repo_config_parses_all_url_forms() {
  let github = || repo_config(Some("github"), Some("github.com"), Some("owner/repo"));
  let cases: [(&str, RepoConfig); 8] = [
    // https / ssh URL（.git 后缀剥离）
    ("https://github.com/owner/repo.git", github()),
    ("ssh://git@github.com/owner/repo.git", github()),
    ("https://github.com/owner/repo", github()),
    // scp-like（user 段丢弃）
    ("git@github.com:owner/repo.git", github()),
    // 裸 owner/repo 缺省 github
    ("owner/repo", github()),
    (
      "https://gitlab.com/owner/repo",
      repo_config(Some("gitlab"), Some("gitlab.com"), Some("owner/repo")),
    ),
    (
      "https://bitbucket.org/owner/repo.git",
      repo_config(Some("bitbucket"), Some("bitbucket.org"), Some("owner/repo")),
    ),
    // 仓库名含点（非 .git 结尾）保留
    (
      "https://github.com/owner/re.po.git",
      repo_config(Some("github"), Some("github.com"), Some("owner/re.po")),
    ),
  ];
  for (input, expected) in cases {
    assert_eq!(get_repo_config(input), expected, "输入：{input}");
  }
}

#[test]
fn repo_config_self_hosted_and_nested_groups() {
  // scp-like 自托管：provider 按原样保留，domain 同名（changelogen 缺省映射外行为）
  assert_eq!(
    get_repo_config("git@gitlab.company.com:owner/repo.git"),
    repo_config(
      Some("gitlab.company.com"),
      Some("gitlab.company.com"),
      Some("owner/repo")
    )
  );
  // https 自托管：provider 不识别 → None
  assert_eq!(
    get_repo_config("https://gitlab.company.com/owner/repo.git"),
    repo_config(None, Some("gitlab.company.com"), Some("owner/repo"))
  );
  // 嵌套 group 路径保留
  assert_eq!(
    get_repo_config("https://gitlab.com/owner/group/repo"),
    repo_config(Some("gitlab"), Some("gitlab.com"), Some("owner/group/repo"))
  );
}

#[test]
fn repo_config_unparseable_returns_all_none() {
  assert_eq!(get_repo_config("???"), repo_config(None, None, None));
  assert_eq!(get_repo_config(""), repo_config(None, None, None));
  // 无 scheme 无冒号：changelogen 同样落入全 undefined（regex 与 new URL 均不吃）
  assert_eq!(
    get_repo_config("github.com/owner/repo"),
    repo_config(None, None, None)
  );
  // 第二段恰为 .git / 空第二段：changelogen 的负向断言与 + 量词整体不匹配
  assert_eq!(get_repo_config("owner/.git"), repo_config(None, None, None));
  assert_eq!(get_repo_config("owner/"), repo_config(None, None, None));
}

#[test]
fn repo_config_allows_single_char_repo_name() {
  assert_eq!(
    get_repo_config("owner/a"),
    repo_config(Some("github"), Some("github.com"), Some("owner/a"))
  );
}

#[test]
fn resolve_repo_prefers_package_json_repository_string() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(
    path.join("package.json"),
    r#"{ "repository": "https://github.com/owner/repo.git" }"#,
  )
  .unwrap();
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(
      Some("github"),
      Some("github.com"),
      Some("owner/repo")
    ))
  );
}

#[test]
fn resolve_repo_reads_repository_object_url() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(
    path.join("package.json"),
    r#"{ "repository": { "url": "git@gitlab.com:owner/repo.git" } }"#,
  )
  .unwrap();
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(
      Some("gitlab"),
      Some("gitlab.com"),
      Some("owner/repo")
    ))
  );
}

#[test]
fn resolve_repo_falls_back_to_git_remote() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir); // init_repo 的 package.json 无 repository 键
  git(
    &path,
    &["remote", "add", "origin", "git@github.com:owner/repo.git"],
  );
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(
      Some("github"),
      Some("github.com"),
      Some("owner/repo")
    ))
  );
}

#[test]
fn resolve_repo_invalid_package_json_falls_through_to_remote() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("package.json"), "{ invalid json").unwrap();
  git(
    &path,
    &["remote", "add", "origin", "https://gitlab.com/owner/repo"],
  );
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(
      Some("gitlab"),
      Some("gitlab.com"),
      Some("owner/repo")
    ))
  );
}

#[test]
fn resolve_repo_without_any_source_returns_none() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  assert_eq!(resolve_repo_config(&path), None);
}

#[test]
fn resolve_repo_empty_repository_object_short_circuits() {
  // changelogen quirk：repository 键存在但无 url → 返回全 None 配置，不再查 remote
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("package.json"), r#"{ "repository": {} }"#).unwrap();
  git(
    &path,
    &["remote", "add", "origin", "git@github.com:owner/repo.git"],
  );
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(None, None, None))
  );
}

#[test]
fn resolve_repo_empty_string_repository_falls_to_remote() {
  // changelogen truthiness："" 为 falsy → 落到 git remote 兜底
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("package.json"), r#"{ "repository": "" }"#).unwrap();
  git(
    &path,
    &["remote", "add", "origin", "git@gitlab.com:owner/repo.git"],
  );
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(
      Some("gitlab"),
      Some("gitlab.com"),
      Some("owner/repo")
    ))
  );
}

#[test]
fn resolve_repo_non_string_truthy_repository_short_circuits() {
  // changelogen truthiness：42 为 truthy 且非 string → 取 .url 得 undefined → 短路全 None
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("package.json"), r#"{ "repository": 42 }"#).unwrap();
  git(
    &path,
    &["remote", "add", "origin", "git@github.com:owner/repo.git"],
  );
  assert_eq!(
    resolve_repo_config(&path),
    Some(repo_config(None, None, None))
  );
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

// ---------------------------------------------------------------------------
// gitignore 批量裁决（收集层）与已跟踪过滤（commit 兜底层）
// ---------------------------------------------------------------------------

#[test]
fn check_ignored_returns_ignored_subset_in_repo() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join(".gitignore"), "residue/\n*.log\n").unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "ignore rules"]);
  let candidates = vec![
    "package.json".to_string(),
    "residue/Cargo.toml".to_string(),
    "build.log".to_string(),
  ];
  let ignored = check_ignored(&path, &candidates).unwrap();
  assert_eq!(
    ignored,
    vec!["residue/Cargo.toml".to_string(), "build.log".to_string()],
    "仅 gitignore 命中项"
  );
}

#[test]
fn check_ignored_outside_git_repo_fails_open() {
  // 非 git 仓库 / 子进程失败 → None（调用方 fail-open 回落不过滤）
  let dir = TempDir::new().unwrap();
  fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
  let candidates = vec!["target/x.json".to_string()];
  assert_eq!(check_ignored(dir.path(), &candidates), None);
}

#[test]
fn filter_tracked_returns_tracked_subset() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::create_dir_all(path.join("nested")).unwrap();
  fs::write(
    path.join("nested/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  let tracked = path.join("package.json").to_string_lossy().into_owned();
  let untracked = path
    .join("nested/package.json")
    .to_string_lossy()
    .into_owned();
  let candidates = vec![tracked.clone(), untracked];
  assert_eq!(
    filter_tracked(&path, &candidates),
    Some(vec![tracked]),
    "仅已跟踪文件留存"
  );
}

#[test]
fn filter_tracked_outside_git_repo_fails_open() {
  let dir = TempDir::new().unwrap();
  let candidates = vec![dir.path().join("a.json").to_string_lossy().into_owned()];
  assert_eq!(filter_tracked(dir.path(), &candidates), None);
}

#[test]
fn empty_candidates_short_circuit() {
  // 空输入不触子进程（git ls-files 无 pathspec 会列出全仓——语义陷阱）
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  assert_eq!(check_ignored(&path, &[]), Some(vec![]));
  assert_eq!(filter_tracked(&path, &[]), Some(vec![]));
}
