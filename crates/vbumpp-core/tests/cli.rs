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
  // JS parity：cac 会把 --output 当全局选项吞掉，name 缺省 → 用法错误
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set", "--output"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token set <name>"), "{err}");
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
