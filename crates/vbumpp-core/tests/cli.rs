//! CLI 应用层（ADR-0016）：token 子命令通路 + bump 默认命令通路 + 全局 flag
//! 的行为矩阵。store 路径注入走临时目录，不触碰真实 Token 存储；消息文案
//! 逐条对齐 JS 时代 cli.ts 的 consola 输出（parity 基准）。

mod common;

use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;
use vbumpp_core::cli::{run_at, RunEnv, TokenPrompt};
use vbumpp_core::token::{read_token_store_at, save_token_at, TokenError};

fn store_in(dir: &TempDir) -> PathBuf {
  dir.path().join("tokens.bin")
}

/// 跑一轮 CLI，收集 stdout / stderr / 退出码（非 TTY 下 style 自动降级纯文本）
fn run(argv: &[&str], store: &Path) -> (String, String, i32) {
  run_full(argv, None, store, None, None)
}

/// 带 provider / cwd / prompt 注入的完整形态（bump 通路与 token set 测试用）
fn run_full(
  argv: &[&str],
  provider: Option<&str>,
  store: &Path,
  cwd: Option<&Path>,
  prompt: Option<TokenPrompt<'_>>,
) -> (String, String, i32) {
  let argv: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
  let env = RunEnv {
    store: Some(store),
    cwd,
    prompt,
  };
  let mut out = Vec::new();
  let mut err = Vec::new();
  let code = run_at(&argv, provider, &env, &mut out, &mut err);
  (
    String::from_utf8(out).unwrap(),
    String::from_utf8(err).unwrap(),
    code,
  )
}

#[test]
fn version_flag_prints_crate_version() {
  let dir = TempDir::new().unwrap();
  let (out, err, code) = run(&["--version"], &store_in(&dir));
  assert_eq!(code, 0, "退出码");
  assert_eq!(
    out.trim(),
    format!("vbumpp {}", env!("CARGO_PKG_VERSION")),
    "--version 取 crate 版本号（ADR-0003 同步）"
  );
  assert!(err.is_empty(), "{err}");
}

#[test]
fn help_flag_lists_all_commands() {
  let dir = TempDir::new().unwrap();
  let (out, _err, code) = run(&["--help"], &store_in(&dir));
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("[...files]"), "{out}");
  assert!(out.contains("token <action> [name]"), "{out}");
  assert!(out.contains("set / list / remove"), "{out}");
  assert!(out.contains("-o, --output"), "{out}");
  assert!(out.contains("-r, --recursive"), "{out}");
  assert!(out.contains("-h, --help"), "{out}");
  assert!(out.contains("-v, --version"), "{out}");
}

#[test]
fn short_help_and_version_flags() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let (out, _, code) = run(&["-h"], &store);
  assert_eq!(code, 0, "-h 退出码");
  assert!(out.contains("usage"), "{out}");
  let (out, _, code) = run(&["-v"], &store);
  assert_eq!(code, 0, "-v 退出码");
  assert!(out.contains(env!("CARGO_PKG_VERSION")), "{out}");
}

#[test]
fn unknown_flag_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["--wat"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unknown option: --wat"), "{err}");
}

#[test]
fn output_flag_missing_value_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["--output"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("option --output requires a value"), "{err}");
}

// ---------------------------------------------------------------------------
// bump 通路（空目录 cwd 直达编排层首错——证明解析、overrides 与 provider
// 穿线全链路接通；解析细节由 src/cli.rs 模块内单元测试锚定）
// ---------------------------------------------------------------------------

#[test]
fn bump_reaches_orchestration_and_surfaces_error() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let cwd = TempDir::new().unwrap();
  let (_out, err, code) = run_full(&[], None, &store_in(&dir), Some(cwd.path()), None);
  assert_eq!(code, 1, "空目录 bump 必败，退出码");
  assert!(!err.is_empty(), "编排层错误应透出到 stderr");
}

#[test]
fn bump_rejects_unknown_provider_before_orchestration() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let cwd = TempDir::new().unwrap();
  let (_out, err, code) = run_full(
    &["-r"],
    Some("bogus"),
    &store_in(&dir),
    Some(cwd.path()),
    None,
  );
  assert_eq!(code, 1, "退出码");
  assert!(
    err.contains("unknown provider: bogus (expected github / gitlab / gitee / gitcode)"),
    "{err}"
  );
}

#[test]
fn bump_accepts_valid_provider_and_reaches_orchestration() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let cwd = TempDir::new().unwrap();
  let (_out, err, code) = run_full(&[], Some("github"), &store_in(&dir), Some(cwd.path()), None);
  assert_eq!(code, 1, "空目录 bump 必败，退出码");
  assert!(
    !err.contains("unknown provider"),
    "合法 provider 应在编排层报错而非解析层：{err}"
  );
}

#[test]
fn bump_with_config_release_runs_non_interactive_end_to_end() {
  // COL-60 票据原场景：.vbumpprc.toml 配 release + confirm=false，
  // CLI bump 全程零交互完成发版（非 TTY 测试环境有任何 prompt 都会失败）
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let cwd = TempDir::new().unwrap();
  let path = common::init_bump_repo(&cwd, "release = \"minor\"\nconfirm = false\npush = false\n");

  let (_out, err, code) = run_full(&[], None, &store_in(&dir), Some(&path), None);
  assert_eq!(code, 0, "非交互 bump 应成功：{err}");
  let pkg = std::fs::read_to_string(path.join("package.json")).unwrap();
  assert!(pkg.contains("1.1.0"), "package.json 应已更新：{pkg}");
  common::git(&path, &["rev-parse", "--verify", "refs/tags/v1.1.0"]);
  assert!(
    path.join("CHANGELOG.md").is_file(),
    "有 tag 应生成 changelog"
  );
}

#[test]
fn token_path_ignores_provider() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let (out, _err, code) = run_full(&["token", "list"], Some("bogus"), &store, None, None);
  assert_eq!(code, 0, "provider 对 token 通路无影响，退出码");
  assert!(out.contains("no tokens configured"), "{out}");
}

// ---------------------------------------------------------------------------
// token 子命令
// ---------------------------------------------------------------------------

#[test]
fn token_bare_errors_usage() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token <action> [name]"), "{err}");
}

#[test]
fn token_unknown_action_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "peek"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(
    err.contains("unknown action: peek (expected set / list / remove)"),
    "{err}"
  );
}

#[test]
fn token_set_without_name_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token set <name>"), "{err}");
}

#[test]
fn token_set_dash_prefixed_name_errors_usage() {
  // token 子命令 flag 扫描：声明名单外的 `--x` 一律按未知 flag 报错（exit 1）
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set", "--output"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unknown option: --output"), "{err}");
}

#[test]
fn token_set_cancelled_warns_without_writing() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(None);
  let (out, _err, code) = run_full(
    &["token", "set", "github"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "取消不是失败，退出码");
  assert!(out.contains("entry canceled"), "{out}");
  assert!(
    read_token_store_at(&store).unwrap().is_empty(),
    "取消不落盘"
  );
}

#[test]
fn token_set_saves_and_reports() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("secret-token".to_string()));
  let (out, _err, code) = run_full(
    &["token", "set", "github"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("github token saved (encrypted)"), "{out}");
  assert_eq!(
    read_token_store_at(&store).unwrap()["github"],
    "secret-token",
    "明文经加密存储可回读"
  );
}

#[test]
fn token_set_prompt_error_surfaces() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| {
    Err(TokenError::Prompt {
      message: "token must not be empty".to_string(),
    })
  };
  let (_out, err, code) = run_full(
    &["token", "set", "github"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("token must not be empty"), "{err}");
}

#[test]
fn token_list_empty_store() {
  let dir = TempDir::new().unwrap();
  let (out, _err, code) = run(&["token", "list"], &store_in(&dir));
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no tokens configured"), "{out}");
}

#[test]
fn token_list_prints_all_names() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "github", "a").unwrap();
  save_token_at(&store, "gitee", "b").unwrap();
  let (out, _err, code) = run(&["token", "list"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitee"), "{out}");
  assert!(out.contains("github"), "{out}");
}

// ---------------------------------------------------------------------------
// token set / list --host（host 作用域键）
// ---------------------------------------------------------------------------

#[test]
fn token_set_gitlab_with_host_saves_scoped_key() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("scoped-secret".to_string()));
  let (out, _err, code) = run_full(
    &["token", "set", "gitlab", "--host", "https://gitlab-a.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "退出码");
  assert!(
    out.contains("gitlab (https://gitlab-a.com) token saved (encrypted)"),
    "{out}"
  );
  assert_eq!(
    read_token_store_at(&store).unwrap()["gitlab@https://gitlab-a.com"],
    "scoped-secret",
    "存储内出现 host 作用域复合键"
  );
}

#[test]
fn token_set_host_prompt_names_target_host() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |name: &str| {
    assert_eq!(
      name, "gitlab (https://gitlab-a.com)",
      "prompt 文案指明目标 host"
    );
    Ok(Some("x".to_string()))
  };
  let (_out, _err, code) = run_full(
    &["token", "set", "gitlab", "--host", "gitlab-a.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "退出码");
}

#[test]
fn token_set_host_normalization_collapses_to_same_key() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("x".to_string()));
  for raw in [
    "gitlab-a.com",
    "https://gitlab-a.com/",
    "HTTPS://GitLab-A.com",
  ] {
    let (_out, _err, code) = run_full(
      &["token", "set", "gitlab", "--host", raw],
      None,
      &store,
      None,
      Some(&prompt),
    );
    assert_eq!(code, 0, "{raw} 退出码");
  }
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens.len(), 1, "三种写法归一到同一键，实际 {tokens:?}");
  assert!(tokens.contains_key("gitlab@https://gitlab-a.com"));
}

#[test]
fn token_set_host_equals_form_and_double_dash_separator() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("x".to_string()));
  // `--host=H` 等值形态
  let (_out, _err, code) = run_full(
    &["token", "set", "gitlab", "--host=https://gitlab-b.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "--host=H 退出码");
  // `--` 之后一律位置参数（`--host` 不再解析为 flag）
  let (_out, _err, code) = run_full(
    &["token", "set", "--", "gitee", "--host=https://gitlab-c.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "-- 分隔退出码");
  let tokens = read_token_store_at(&store).unwrap();
  assert!(
    tokens.contains_key("gitlab@https://gitlab-b.com"),
    "{tokens:?}"
  );
  assert!(
    tokens.contains_key("gitee"),
    "-- 后 --host 按位置参数忽略，gitee 落 provider 级键：{tokens:?}"
  );
}

#[test]
fn token_set_host_rejects_non_gitlab_providers() {
  for provider in ["github", "gitee", "gitcode"] {
    let dir = TempDir::new().unwrap();
    let store = store_in(&dir);
    let prompt = |_name: &str| Ok(Some("x".to_string()));
    let (_out, err, code) = run_full(
      &["token", "set", provider, "--host", "https://gitlab-a.com"],
      None,
      &store,
      None,
      Some(&prompt),
    );
    assert_eq!(code, 1, "{provider} 退出码");
    assert!(
      err.contains("--host is only supported for gitlab"),
      "{provider}: {err}"
    );
    assert!(
      read_token_store_at(&store).unwrap().is_empty(),
      "{provider} 拒绝路径不落盘"
    );
  }
}

#[test]
fn token_set_host_missing_value_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set", "gitlab", "--host"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("option --host requires a value"), "{err}");
}

#[test]
fn token_set_host_invalid_value_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(
    &["token", "set", "gitlab", "--host", "https://"],
    &store_in(&dir),
  );
  assert_eq!(code, 1, "退出码");
  assert!(
    err.contains("invalid host: https:// (missing host name)"),
    "{err}"
  );
}

#[test]
fn token_list_shows_friendly_scoped_names() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-a.com", "scoped").unwrap();
  let (out, _err, code) = run(&["token", "list"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  assert!(
    out.lines().any(|line| line.ends_with("gitlab")),
    "provider 级键按原名显示：{out}"
  );
}

#[test]
fn token_list_host_filters_single_entry() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-a.com", "a").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-b.com", "b").unwrap();
  // 过滤值同样经规范化（无 scheme 写法可命中）
  let (out, _err, code) = run(&["token", "list", "--host", "gitlab-a.com"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  assert!(!out.contains("gitlab-b.com"), "{out}");
  assert!(
    !out.lines().any(|line| line.ends_with("gitlab")),
    "provider 级键被过滤：{out}"
  );
}

#[test]
fn token_list_host_filter_without_match_warns() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  let (out, _err, code) = run(&["token", "list", "--host", "gitlab-a.com"], &store);
  assert_eq!(code, 0, "未命中非失败，退出码");
  assert!(
    out.contains("no token found for host https://gitlab-a.com"),
    "{out}"
  );
}

#[test]
fn token_list_unknown_flag_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "list", "--wat"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unknown option: --wat"), "{err}");
}

#[test]
fn token_remove_without_name_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "remove"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token remove <name>"), "{err}");
}

#[test]
fn token_remove_absent_warns_and_keeps_store() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "x").unwrap();
  let (out, _err, code) = run(&["token", "remove", "github"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no token found for github"), "{out}");
  assert_eq!(
    read_token_store_at(&store).unwrap().len(),
    1,
    "误删不存在键不动存储"
  );
}

#[test]
fn token_remove_existing_succeeds() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "github", "x").unwrap();
  let (out, _err, code) = run(&["token", "remove", "github"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("github token removed"), "{out}");
  assert!(
    read_token_store_at(&store).unwrap().is_empty(),
    "删除后存储为空"
  );
}
