//! CLI 应用层：argv 语法（子命令、flag、help 文案、错误提示、
//! 退出码）的唯一归属。手写解析器（语法盘子小，不引 clap）；npm bin 与原生
//! CLI 二进制（`crates/vbumpp`）为共享 `run_from_argv` 的两个
//! 薄壳前端。
//!
//! 消息文案逐条对齐 JS 时代 cli.ts 的 consola 输出（parity 基准）；着色仿
//! `progress.rs` 先例（dialoguer `console::style`，非 TTY 自动降级纯文本）。
//!
//! 布局：解析（`parse`）、四个子命令（`bump` / `release` / `schema` / `token`）、
//! 输出行样式（`output`）各一文件；本文件持入口与环境注入签名。

mod bump;
mod output;
mod parse;
mod release;
mod schema;
mod token;

use std::io::{stderr, stdout, Write};
use std::path::Path;

use output::{error_line, print_help, print_version};

use crate::token::TokenError;

// 测试缝（tests/cli/ 直取解析层细节，先例：`release::resolve_token`）：
// `#[doc(hidden)]` 不入公开文档，`cli::{run_from_argv, run_at, RunEnv}`
// 仍为唯一受支持的公开入口
#[doc(hidden)]
pub use bump::{bump_overrides, resolve_provider};
#[doc(hidden)]
pub use parse::{parse, BumpArgs, Command, ReleaseArgs, SchemaArgs};
#[doc(hidden)]
pub use token::scan_token_args;

/// 真实入口：npm bin / 原生 bin 的唯一调用点。`provider` 为平台变体注入身份
/// （bump 与 release 通路生效；token 子命令无视）——argv `--provider` flag
/// 优先于该注入。返回退出码，由调用壳回写
/// （Rust 不越权设宿主进程状态）。
pub fn run_from_argv(argv: &[String], provider: Option<&str>) -> i32 {
  let stdout = stdout();
  let stderr = stderr();
  let mut out = stdout.lock();
  let mut err = stderr.lock();
  let env = RunEnv {
    store: None,
    cwd: None,
    home: None,
    prompt: None,
    confirm: None,
  };
  run_at(argv, provider, &env, &mut out, &mut err)
}

/// token 录入交互的注入签名：返回 Ok(Some(明文)) 保存、Ok(None) 取消、Err 报错
/// （真实实现为 dialoguer 密码 prompt）
pub type TokenPrompt<'a> = &'a dyn Fn(&str) -> Result<Option<String>, TokenError>;

/// token remove 二次确认的注入签名：返回 Ok(true) 确认删除、Ok(false) 拒绝
/// （取消，exit 0）、Err 无法交互（非 TTY——报错引导 --yes，exit 1）。
/// 真实实现为 dialoguer Confirm（默认 No）
pub type ConfirmPrompt<'a> = &'a dyn Fn(&str) -> Result<bool, TokenError>;

/// 可测内核的环境注入：`store` 覆盖 token 存储路径（None 走环境解析，
/// `VBUMPP_TOKEN_STORE` → `VBUMPP_HOME` → 系统 home）；`cwd` 覆盖
/// bump 执行目录（None 取进程当前目录）；`home` 覆盖全局配置目录（None 走
/// `VBUMPP_HOME` → `~/.vbumpp`——schema `--global` 落点消费）；`prompt` 覆盖
/// token 录入交互（None 走 dialoguer 密码 prompt）；`confirm` 覆盖
/// remove 二次确认（None 走 dialoguer Confirm + TTY 守卫）。
pub struct RunEnv<'a> {
  pub store: Option<&'a Path>,
  pub cwd: Option<&'a Path>,
  pub home: Option<&'a Path>,
  pub prompt: Option<TokenPrompt<'a>>,
  pub confirm: Option<ConfirmPrompt<'a>>,
}

/// 可测内核：`out` / `err` 注入输出汇。正常消息进 `out`（consola
/// info/success/warn parity 走 stdout），错误进 `err`（consola.error parity
/// 走 stderr）。
pub fn run_at(
  argv: &[String],
  provider: Option<&str>,
  env: &RunEnv,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  match parse(argv) {
    Ok(Command::Help) => print_help(out),
    Ok(Command::Version) => print_version(out),
    Ok(Command::Token(args)) => token::token_command(&args, env, out, err),
    Ok(Command::Bump(args)) => bump::bump_command(&args, provider, env, out, err),
    Ok(Command::Release(args)) => release::release_command(&args, provider, env, out, err),
    Ok(Command::Schema(args)) => schema::schema_command(&args, env, out, err),
    Err(message) => {
      error_line(err, &message);
      1
    }
  }
}
