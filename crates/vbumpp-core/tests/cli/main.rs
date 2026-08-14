//! CLI 应用层（ADR-0016）：全局 flag 与 bump 默认命令通路的行为矩阵
//! （解析层与 token 子命令用例在子模块，镜像 src/cli/ 切面）。store 路径
//! 注入走临时目录，不触碰真实 Token 存储；消息文案逐条对齐 JS 时代 cli.ts
//! 的 consola 输出（parity 基准）。

#[path = "../common.rs"]
mod common;

mod parse;
mod token;

use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;
use vbumpp_core::cli::{run_at, ConfirmPrompt, RunEnv, TokenPrompt};

fn argv(items: &[&str]) -> Vec<String> {
  items.iter().map(|s| (*s).to_string()).collect()
}

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
    confirm: None,
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

/// token remove 矩阵用例：confirm 交互注入（None 走真实 TTY 守卫实现——
/// 测试环境非 TTY 必报错，矩阵用例一律显式注入或经 --yes / --dry-run 绕过）
fn run_remove(
  argv: &[&str],
  store: &Path,
  confirm: Option<ConfirmPrompt<'_>>,
) -> (String, String, i32) {
  let argv: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
  let env = RunEnv {
    store: Some(store),
    cwd: None,
    prompt: None,
    confirm,
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
// 穿线全链路接通；解析细节由 parse 子模块锚定）
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
