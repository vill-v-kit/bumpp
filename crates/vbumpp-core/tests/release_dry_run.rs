//! release --dry-run（COL-84）：前置校验照走（报错文案与真实执行一致、exit 1，
//! 可当 CI 预检门禁）+ token 来源报告（缺失降级为警告行、exit 0）+ 平台
//! Release 计划预览（provider / host / owner/repo / tag_name / prerelease /
//! changelog 版本节全文 / 拦截到的请求行）。全程零网络请求（含 gitlab 的
//! GET project id）；明文 token 不出现在任何输出行（gitcode 的 query 注入
//! 形态经 `[redacted]` 脱敏）。
//!
//! env 修改为进程全局：本文件全部用例经 ENV_LOCK 串行，入场先净化 provider
//! token 变量与 VBUMPP_TOKEN_STORE，隔离并发竞态（tests/effects.rs 同先例）。
//!
//! 位置说明：本文件是 CLI 应用层通路测试（经 `run_at` 全链路，对齐
//! tests/cli.rs 定位），非 provider 行为测试——故不进 tests/release/ 镜像。

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use tempfile::TempDir;
use vbumpp_core::cli::{run_at, RunEnv};
use vbumpp_core::token::save_token_at;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 入场串行 + 净化：清掉全部 provider token 环境变量与存储路径覆盖；
/// 全局配置目录指向不存在路径（token 存储随之落空，按空表继续）
fn sanitized_env() -> MutexGuard<'static, ()> {
  let guard = ENV_LOCK.lock().unwrap();
  for key in common::PROVIDER_TOKEN_ENV_VARS {
    std::env::remove_var(key);
  }
  std::env::remove_var("VBUMPP_TOKEN_STORE");
  common::isolate_global_home();
  guard
}

/// 跑一轮 CLI，cwd 注入仓库目录，收集 stdout / stderr / 退出码
/// （release 链的 token 存储走环境解析，RunEnv.store 仅满足注入签名）
fn run_release(argv: &[&str], cwd: &Path) -> (String, String, i32) {
  let argv: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
  let store = cwd.join("tokens.bin");
  let env = RunEnv {
    store: Some(&store),
    cwd: Some(cwd),
    prompt: None,
    confirm: None,
  };
  let mut out = Vec::new();
  let mut err = Vec::new();
  let code = run_at(&argv, None, &env, &mut out, &mut err);
  (
    String::from_utf8(out).unwrap(),
    String::from_utf8(err).unwrap(),
    code,
  )
}

/// release 校验通过形态的最小仓库：remote（owner/repo 推断来源）+ CHANGELOG.md
/// 含 `## v{version}` 节 + 对应 tag。changelog 另带 v1.0.0 旧节（全文预览
/// 不得混入）
fn init_release_repo(dir: &TempDir, version: &str, remote: &str) -> PathBuf {
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  common::git(&path, &["config", "commit.gpgsign", "false"]);
  common::git(&path, &["remote", "add", "origin", remote]);
  std::fs::write(
    path.join("CHANGELOG.md"),
    format!(
      "# Changelog\n\n## v{version}\n\n### Features\n\n- add x\n\n\
       ## v1.0.0\n\n### Bug Fixes\n\n- old thing\n"
    ),
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  common::git(&path, &["tag", &format!("v{version}")]);
  path
}

// ---------------------------------------------------------------------------
// 校验照走：报错文案与真实执行逐字节一致、exit 1
// ---------------------------------------------------------------------------

#[test]
fn missing_tag_fails_identically_to_real_run() {
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  std::fs::write(path.join("CHANGELOG.md"), "# Changelog\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);

  let (_o, err_dry, code_dry) = run_release(
    &["release", "9.9.9", "--provider", "gitee", "--dry-run"],
    &path,
  );
  let (_o, err_real, code_real) = run_release(&["release", "9.9.9", "--provider", "gitee"], &path);
  assert_eq!(code_dry, 1, "dry-run 校验失败照常 exit 1");
  assert_eq!(code_real, 1, "真实执行基线 exit 1");
  assert_eq!(err_dry, err_real, "报错文案与真实执行逐字节一致");
  assert!(
    err_dry.contains(
      "tag v9.9.9 not found locally — run the bump flow first (release requires an existing tag)"
    ),
    "{err_dry}"
  );
}

#[test]
fn missing_changelog_section_fails_identically_to_real_run() {
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitee.com:owner/repo.git");
  // 请求一个 changelog 中不存在版本节的已打 tag
  common::git(&path, &["tag", "v3.0.0"]);

  let (_o, err_dry, code_dry) = run_release(
    &["release", "3.0.0", "--provider", "gitee", "--dry-run"],
    &path,
  );
  let (_o, err_real, code_real) = run_release(&["release", "3.0.0", "--provider", "gitee"], &path);
  assert_eq!(code_dry, 1, "dry-run 校验失败照常 exit 1");
  assert_eq!(code_real, 1, "真实执行基线 exit 1");
  assert_eq!(err_dry, err_real, "报错文案与真实执行逐字节一致");
  assert!(
    err_dry.contains("no changelog section found for v3.0.0 in CHANGELOG.md"),
    "{err_dry}"
  );
}

// ---------------------------------------------------------------------------
// token 来源报告 + 四家 provider 预览
// ---------------------------------------------------------------------------

#[test]
fn store_token_source_and_gitee_preview() {
  let _guard = sanitized_env();
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitee.com:owner/repo.git");
  let store = dir.path().join("tokens.bin");
  save_token_at(&store, "gitee", "store-token-xyz").unwrap();
  std::env::set_var("VBUMPP_TOKEN_STORE", &store);

  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "gitee", "--dry-run"],
    &path,
  );
  assert_eq!(code, 0, "校验通过 exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  assert!(out.contains("release plan"), "{out}");
  assert!(out.contains("token source: token store"), "{out}");
  assert!(out.contains("provider: Gitee"), "{out}");
  assert!(out.contains("host: https://gitee.com/api/v5"), "{out}");
  assert!(out.contains("repo: owner/repo"), "{out}");
  assert!(out.contains("tag_name: v2.0.0"), "{out}");
  assert!(out.contains("prerelease: false"), "{out}");
  // body 即提取的 changelog 版本节全文；其他版本节不混入
  assert!(out.contains("- add x"), "{out}");
  assert!(!out.contains("old thing"), "{out}");
  // 拦截到的请求行：URL 与真实执行一致
  assert!(
    out.contains("POST https://gitee.com/api/v5/repos/owner/repo/releases"),
    "{out}"
  );
  // 明文 token 零泄漏（gitee 注入请求体 access_token，不进入任何输出行）
  assert!(!out.contains("store-token-xyz"), "{out}");
}

#[test]
fn env_token_source_and_gitlab_preview() {
  let _guard = sanitized_env();
  std::env::set_var("GITLAB_TOKEN", "env-token-gl");
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitlab.com:owner/repo.git");

  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "gitlab", "--dry-run"],
    &path,
  );
  assert_eq!(code, 0, "校验通过 exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  assert!(
    out.contains("token source: environment variable GITLAB_TOKEN"),
    "{out}"
  );
  assert!(out.contains("provider: Gitlab"), "{out}");
  assert!(out.contains("host: https://gitlab.com"), "{out}");
  assert!(out.contains("repo: owner/repo"), "{out}");
  // GET project id 同样拦截：预览显示 url 编码的 owner/repo 路径
  assert!(
    out.contains("GET https://gitlab.com/api/v4/projects/owner%2Frepo"),
    "{out}"
  );
  assert!(
    out.contains("POST https://gitlab.com/api/v4/projects/"),
    "{out}"
  );
  assert!(!out.contains("env-token-gl"), "{out}");
  std::env::remove_var("GITLAB_TOKEN");
}

#[test]
fn gh_cli_token_source_and_github_preview() {
  let _guard = sanitized_env();
  // PATH 前置一枚假 gh：`gh auth token` 输出固定 token（github 兜底链）
  let bin = TempDir::new().unwrap();
  let gh = bin.path().join("gh");
  std::fs::write(&gh, "#!/bin/sh\necho fake-gh-token\n").unwrap();
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
  }
  let old_path = std::env::var("PATH").unwrap_or_default();
  std::env::set_var("PATH", format!("{}:{old_path}", bin.path().display()));

  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@github.com:owner/repo.git");
  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "github", "--dry-run"],
    &path,
  );
  std::env::set_var("PATH", old_path);

  assert_eq!(code, 0, "校验通过 exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  assert!(
    out.contains("token source: gh CLI (`gh auth token`)"),
    "{out}"
  );
  assert!(out.contains("provider: Github"), "{out}");
  assert!(out.contains("host: https://api.github.com"), "{out}");
  assert!(
    out.contains("POST https://api.github.com/repos/owner/repo/releases"),
    "{out}"
  );
  // github 注入 authorization 头，不进入任何输出行
  assert!(!out.contains("fake-gh-token"), "{out}");
}

#[test]
fn gitcode_preview_redacts_query_token() {
  let _guard = sanitized_env();
  // gitcode 经 query 注入 token（form 编码）：预览请求行必须脱敏
  std::env::set_var("GITCODE_TOKEN", "s3cr3t+key=");
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitcode.com:owner/repo.git");

  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "gitcode", "--dry-run"],
    &path,
  );
  std::env::remove_var("GITCODE_TOKEN");

  assert_eq!(code, 0, "校验通过 exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  assert!(
    out.contains("token source: environment variable GITCODE_TOKEN"),
    "{out}"
  );
  assert!(
    out.contains("host: https://api.gitcode.com/api/v5"),
    "{out}"
  );
  assert!(
    out.contains("POST https://api.gitcode.com/api/v5/repos/owner/repo/releases"),
    "{out}"
  );
  assert!(out.contains("[redacted]"), "{out}");
  // 原始与 form 编码两形态都不得出现
  assert!(!out.contains("s3cr3t"), "{out}");
}

#[test]
fn missing_token_warns_and_still_previews() {
  let _guard = sanitized_env();
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitee.com:owner/repo.git");

  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "gitee", "--dry-run"],
    &path,
  );
  assert_eq!(code, 0, "token 缺失降级为警告，预览照常 exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  // 警告行复用真实执行的报错文案（仅降级不改动措辞）
  assert!(
    out.contains("no Gitee token detected; run vbumpp token set gitee to add one"),
    "{out}"
  );
  // 预览其余部分照常输出
  assert!(out.contains("tag_name: v2.0.0"), "{out}");
  assert!(
    out.contains("POST https://gitee.com/api/v5/repos/owner/repo/releases"),
    "{out}"
  );
}

#[test]
fn gitlab_scoped_store_token_reports_store_source() {
  let _guard = sanitized_env();
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitlab.com:owner/repo.git");
  // 配置自建 host + host 作用域键：dry-run 的宽容解析链与真实执行同路消费
  std::fs::write(
    path.join(".vbumpprc.json"),
    "{\n  \"gitlab\": {\n    \"host\": \"https://gitlab-a.com\"\n  }\n}\n",
  )
  .unwrap();
  let store = dir.path().join("tokens.bin");
  save_token_at(&store, "gitlab@https://gitlab-a.com", "scoped-token-gl").unwrap();
  std::env::set_var("VBUMPP_TOKEN_STORE", &store);

  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "gitlab", "--dry-run"],
    &path,
  );
  assert_eq!(code, 0, "校验通过 exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  assert!(out.contains("token source: token store"), "{out}");
  assert!(out.contains("host: https://gitlab-a.com"), "{out}");
  assert!(
    !out.contains("no Gitlab token detected"),
    "scoped 键命中不得报缺失：{out}"
  );
  assert!(!out.contains("scoped-token-gl"), "{out}");
}

#[test]
fn gitlab_missing_token_warns_with_host_guidance() {
  let _guard = sanitized_env();
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitlab.com:owner/repo.git");

  let (out, err, code) = run_release(
    &["release", "2.0.0", "--provider", "gitlab", "--dry-run"],
    &path,
  );
  assert_eq!(code, 0, "token 缺失降级为警告，预览照常 exit 0：{err}");
  // 有效 host 缺省 https://gitlab.com——警告行带 host 指引（与真实执行报错同文案）
  assert!(
    out.contains(
      "no Gitlab token detected for https://gitlab.com; \
       run vbumpp token set gitlab --host https://gitlab.com to add one"
    ),
    "{out}"
  );
}

#[test]
fn prerelease_versions_mark_prerelease() {
  let _guard = sanitized_env();
  // prerelease 判定（beta/alpha）：两种前缀都命中同一判定规则
  for version in ["2.0.0-beta.1", "2.0.0-alpha"] {
    let dir = TempDir::new().unwrap();
    let path = init_release_repo(&dir, version, "git@gitee.com:owner/repo.git");

    let (out, err, code) = run_release(
      &["release", version, "--provider", "gitee", "--dry-run"],
      &path,
    );
    assert_eq!(code, 0, "校验通过 exit 0：{err}");
    assert!(out.contains(&format!("tag_name: v{version}")), "{out}");
    assert!(out.contains("prerelease: true"), "{out}");
  }
}

// ---------------------------------------------------------------------------
// flag 组合与 help 文案
// ---------------------------------------------------------------------------

#[test]
fn dry_run_combines_with_custom_output() {
  let _guard = sanitized_env();
  let dir = TempDir::new().unwrap();
  let path = init_release_repo(&dir, "2.0.0", "git@gitee.com:owner/repo.git");
  // -o 指定非默认 changelog 文件：校验与预览都从它提取
  std::fs::rename(path.join("CHANGELOG.md"), path.join("HISTORY.md")).unwrap();

  let (out, err, code) = run_release(
    &[
      "release",
      "2.0.0",
      "--provider",
      "gitee",
      "--dry-run",
      "-o",
      "HISTORY.md",
    ],
    &path,
  );
  assert_eq!(code, 0, "与 -o 组合正常 exit 0：{err}");
  assert!(out.contains("- add x"), "{out}");
}

#[test]
fn help_lists_dry_run_flag() {
  let dir = TempDir::new().unwrap();
  let (out, _err, code) = run_release(&["--help"], dir.path());
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("--dry-run"), "{out}");
}
