//! CLI 应用层（ADR-0016）：argv 语法（子命令、flag、help 文案、错误提示、
//! 退出码）的唯一归属。手写解析器（语法盘子小，不引 clap）；npm bin 与规划
//! 中的原生 CLI 二进制为共享 `run_from_argv` 的两个薄壳前端。
//!
//! 消息文案逐条对齐 JS 时代 cli.ts 的 consola 输出（parity 基准）；着色仿
//! `progress.rs` 先例（dialoguer `console::style`，非 TTY 自动降级纯文本）。

use std::io::Write;
use std::path::{Path, PathBuf};

use dialoguer::console::style;
use serde_json::{json, Map, Value};

use crate::token;

/// 真实入口：npm bin / 未来原生 bin 的唯一调用点。`provider` 为平台变体身份
/// （仅 bump 通路生效；token 子命令无视）。返回退出码，由调用壳回写
/// （Rust 不越权设宿主进程状态）。
pub fn run_from_argv(argv: &[String], provider: Option<&str>) -> i32 {
  let stdout = std::io::stdout();
  let stderr = std::io::stderr();
  let mut out = stdout.lock();
  let mut err = stderr.lock();
  let env = RunEnv {
    store: None,
    cwd: None,
    prompt: None,
  };
  run_at(argv, provider, &env, &mut out, &mut err)
}

/// token 录入交互的注入签名：返回 Ok(Some(明文)) 保存、Ok(None) 取消、Err 报错
/// （真实实现为 dialoguer 密码 prompt，ADR-0014）
pub type TokenPrompt<'a> = &'a dyn Fn(&str) -> Result<Option<String>, token::TokenError>;

/// 可测内核的环境注入：`store` 覆盖 token 存储路径（None 走环境解析，
/// `VBUMPP_TOKEN_STORE` → `VBUMPP_HOME` → 系统 home，ADR-0015）；`cwd` 覆盖
/// bump 执行目录（None 取进程当前目录）；`prompt` 覆盖 token 录入交互
/// （None 走 dialoguer 密码 prompt，ADR-0014）。
pub struct RunEnv<'a> {
  pub store: Option<&'a Path>,
  pub cwd: Option<&'a Path>,
  pub prompt: Option<TokenPrompt<'a>>,
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
    Ok(Command::Token(args)) => token_command(&args, env, out, err),
    Ok(Command::Bump(args)) => bump_command(&args, provider, env, err),
    Err(message) => {
      error_line(err, &message);
      1
    }
  }
}

// ---------------------------------------------------------------------------
// 手写解析器（不引 clap——语法盘子小；未来子命令家族长大再迁，只动本模块）
// ---------------------------------------------------------------------------

/// 解析结果：全局 flag 优先于子命令判定（cac parity——`--help` 出现即显示帮助，
/// 无视其他参数）；`--` 之后一律视为位置参数。
enum Command {
  Help,
  Version,
  Token(Vec<String>),
  Bump(BumpArgs),
}

/// bump 默认命令（`vbumpp [...files]`）的解析产物
struct BumpArgs {
  files: Vec<String>,
  recursive: bool,
  output: String,
}

fn parse(argv: &[String]) -> Result<Command, String> {
  let mut positional_only = false;
  for arg in argv {
    if positional_only {
      continue;
    }
    match arg.as_str() {
      "--" => positional_only = true,
      "-h" | "--help" => return Ok(Command::Help),
      "-v" | "--version" => return Ok(Command::Version),
      _ => {}
    }
  }
  match argv.first().map(String::as_str) {
    Some("token") => Ok(Command::Token(argv[1..].to_vec())),
    _ => parse_bump(argv),
  }
}

fn parse_bump(argv: &[String]) -> Result<Command, String> {
  let mut args = BumpArgs {
    files: Vec::new(),
    recursive: false,
    output: "CHANGELOG.md".to_string(),
  };
  let mut positional_only = false;
  let mut i = 0;
  while i < argv.len() {
    let arg = &argv[i];
    i += 1;
    if positional_only {
      args.files.push(arg.clone());
      continue;
    }
    match arg.as_str() {
      "--" => positional_only = true,
      "-r" | "--recursive" => args.recursive = true,
      "-o" | "--output" => {
        let Some(value) = argv.get(i) else {
          return Err(format!("option {arg} requires a value"));
        };
        args.output = value.clone();
        i += 1;
      }
      _ if arg.starts_with("--") => match arg.split_once('=') {
        Some(("--output", value)) => args.output = value.to_string(),
        // 布尔 flag 带值按 mri 惯例视为 truthy（--recursive=false 亦开启）
        Some(("--recursive", _)) => args.recursive = true,
        _ => return Err(format!("unknown option: {arg}")),
      },
      _ if arg.starts_with('-') && arg.len() > 1 => {
        // 短 flag 簇（mri 惯例可合并，如 `-ro out.md`）
        let cluster = &arg[1..];
        let mut offset = 0;
        for c in cluster.chars() {
          match c {
            'r' => args.recursive = true,
            'h' => return Ok(Command::Help),
            'v' => return Ok(Command::Version),
            'o' => {
              let rest = &cluster[offset + c.len_utf8()..];
              let rest = rest.strip_prefix('=').unwrap_or(rest);
              if !rest.is_empty() {
                args.output = rest.to_string();
              } else {
                let Some(value) = argv.get(i) else {
                  return Err("option -o requires a value".to_string());
                };
                args.output = value.clone();
                i += 1;
              }
              break;
            }
            _ => return Err(format!("unknown option: -{c}")),
          }
          offset += c.len_utf8();
        }
      }
      _ => args.files.push(arg.clone()),
    }
  }
  Ok(Command::Bump(args))
}

// ---------------------------------------------------------------------------
// bump 默认命令
// ---------------------------------------------------------------------------

/// argv → overrides（旧 cli.ts 的 JS 对象构造原样收编）：`recursive` 与
/// `changelog.output` 始终传（cac 默认值语义）；`files` 仅在非空时注入——
/// ADR-0013 浅合并语义，空 files 整体替换掉配置文件的 files 是旧 defu 行为
fn bump_overrides(args: &BumpArgs) -> Map<String, Value> {
  let mut overrides = Map::new();
  if !args.files.is_empty() {
    overrides.insert("files".to_string(), json!(args.files));
  }
  overrides.insert("recursive".to_string(), json!(args.recursive));
  overrides.insert("changelog".to_string(), json!({ "output": args.output }));
  overrides
}

fn bump_command(
  args: &BumpArgs,
  provider: Option<&str>,
  env: &RunEnv,
  err: &mut impl Write,
) -> i32 {
  let provider = match provider
    .map(|p| {
      crate::release::Provider::parse(p)
        .ok_or_else(|| format!("unknown provider: {p} (expected github / gitlab / gitee / gitcode)"))
    })
    .transpose()
  {
    Ok(provider) => provider,
    Err(message) => {
      error_line(err, &message);
      return 1;
    }
  };
  let cwd = match env.cwd {
    Some(path) => path.to_path_buf(),
    None => match std::env::current_dir() {
      Ok(cwd) => cwd,
      Err(e) => {
        error_line(err, &format!("cannot resolve current directory: {e}"));
        return 1;
      }
    },
  };
  let options = crate::orchestrate::BumpVersionOptions {
    overrides: Some(bump_overrides(args)),
    provider,
  };
  match crate::orchestrate::bump_version(&options, &cwd) {
    Ok(_) => 0,
    Err(e) => {
      error_line(err, &e.to_string());
      1
    }
  }
}

// ---------------------------------------------------------------------------
// token 子命令（消息文案与 cli.ts switch 逐条 parity）
// ---------------------------------------------------------------------------

fn token_command(args: &[String], env: &RunEnv, out: &mut impl Write, err: &mut impl Write) -> i32 {
  let Some(action) = args.first() else {
    error_line(
      err,
      "usage: vbumpp token <action> [name] (action: set / list / remove)",
    );
    return 1;
  };
  match action.as_str() {
    "set" => {
      let Some(name) = positional_name(args.get(1)) else {
        error_line(err, "usage: vbumpp token set <name>");
        return 1;
      };
      // 密码交互在 Rust 侧（dialoguer，ADR-0014）
      let prompt = env.prompt.unwrap_or(&token::prompt_token);
      match prompt(name) {
        Ok(Some(plaintext)) => {
          let Some(store) = resolve_store(env.store, err) else {
            return 1;
          };
          match token::save_token_at(&store, name, &plaintext) {
            Ok(()) => {
              success_line(out, &format!("{name} token saved (encrypted)"));
              0
            }
            Err(e) => {
              error_line(err, &e.to_string());
              1
            }
          }
        }
        Ok(None) => {
          warn_line(out, "entry canceled");
          0
        }
        Err(e) => {
          error_line(err, &e.to_string());
          1
        }
      }
    }
    "list" => {
      let Some(store) = resolve_store(env.store, err) else {
        return 1;
      };
      match token::read_token_store_at(&store) {
        Ok(tokens) if tokens.is_empty() => {
          info_line(out, "no tokens configured");
          0
        }
        Ok(tokens) => {
          for name in tokens.keys() {
            info_line(out, name);
          }
          0
        }
        Err(e) => {
          error_line(err, &e.to_string());
          1
        }
      }
    }
    "remove" => {
      let Some(name) = positional_name(args.get(1)) else {
        error_line(err, "usage: vbumpp token remove <name>");
        return 1;
      };
      let Some(store) = resolve_store(env.store, err) else {
        return 1;
      };
      match token::remove_token_at(&store, name) {
        Ok(true) => {
          success_line(out, &format!("{name} token removed"));
          0
        }
        Ok(false) => {
          warn_line(out, &format!("no token found for {name}"));
          0
        }
        Err(e) => {
          error_line(err, &e.to_string());
          1
        }
      }
    }
    other => {
      error_line(
        err,
        &format!("unknown action: {other} (expected set / list / remove)"),
      );
      1
    }
  }
}

/// name 位置参数：`-` 前缀视为 flag 被吞（cac parity——name 缺省即用法错误）
fn positional_name(arg: Option<&String>) -> Option<&str> {
  arg.map(String::as_str).filter(|s| !s.starts_with('-'))
}

/// token 存储路径：注入值优先，缺省走环境解析
fn resolve_store(store: Option<&Path>, err: &mut impl Write) -> Option<PathBuf> {
  match store {
    Some(path) => Some(path.to_path_buf()),
    None => match token::store_path() {
      Ok(path) => Some(path),
      Err(e) => {
        error_line(err, &e.to_string());
        None
      }
    },
  }
}

// ---------------------------------------------------------------------------
// 全局 flag 与输出样式（仿 progress.rs：symbol + console::style 着色）
// ---------------------------------------------------------------------------

fn print_version(out: &mut impl Write) -> i32 {
  let _ = writeln!(out, "vbumpp {}", env!("CARGO_PKG_VERSION"));
  0
}

fn print_help(out: &mut impl Write) -> i32 {
  let _ = writeln!(
    out,
    "usage:\n  \
     vbumpp [...files]                bump version and generate changelog\n  \
     vbumpp token <action> [name]     manage tokens (action: set / list / remove), stored encrypted\n\
     \noptions:\n  \
     -o, --output [output]   where CHANGELOG.md is generated (default CHANGELOG.md)\n  \
     -r, --recursive         recursively\n  \
     -h, --help              show help\n  \
     -v, --version           show version"
  );
  0
}

fn success_line(out: &mut impl Write, msg: &str) {
  let _ = writeln!(out, "{} {msg}", style("✔").green());
}

fn info_line(out: &mut impl Write, msg: &str) {
  let _ = writeln!(out, "{} {msg}", style("ℹ").blue());
}

fn warn_line(out: &mut impl Write, msg: &str) {
  let _ = writeln!(out, "{} {msg}", style("⚠").yellow());
}

fn error_line(err: &mut impl Write, msg: &str) {
  let _ = writeln!(err, "{} {msg}", style("✖").red());
}

#[cfg(test)]
mod tests {
  use super::*;

  fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
  }

  fn parse_bump_args(items: &[&str]) -> BumpArgs {
    match parse(&argv(items)) {
      Ok(Command::Bump(args)) => args,
      other => panic!("应为 Bump，实际 {other:?}"),
    }
  }

  impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Self::Help => f.write_str("Help"),
        Self::Version => f.write_str("Version"),
        Self::Token(args) => write!(f, "Token({args:?})"),
        Self::Bump(args) => write!(f, "Bump(files={:?})", args.files),
      }
    }
  }

  #[test]
  fn dash_prefixed_name_is_not_positional() {
    assert_eq!(positional_name(Some(&"github".to_string())), Some("github"));
    assert_eq!(positional_name(Some(&"--output".to_string())), None);
    assert_eq!(positional_name(None), None);
  }

  #[test]
  fn help_and_version_flags_win_everywhere() {
    assert!(matches!(parse(&argv(&["--help"])), Ok(Command::Help)));
    assert!(matches!(parse(&argv(&["-h"])), Ok(Command::Help)));
    assert!(matches!(
      parse(&argv(&["foo", "--help"])),
      Ok(Command::Help)
    ));
    assert!(matches!(
      parse(&argv(&["token", "list", "-h"])),
      Ok(Command::Help)
    ));
    assert!(matches!(parse(&argv(&["--version"])), Ok(Command::Version)));
    assert!(matches!(parse(&argv(&["-v"])), Ok(Command::Version)));
    // `--` 之后不再识别 flag
    assert!(matches!(
      parse(&argv(&["--", "--help"])),
      Ok(Command::Bump(_))
    ));
  }

  #[test]
  fn token_routes_with_remaining_args() {
    match parse(&argv(&["token", "list"])) {
      Ok(Command::Token(args)) => assert_eq!(args, vec!["list".to_string()]),
      other => panic!("应为 Token，实际 {other:?}"),
    }
  }

  #[test]
  fn bump_parses_positionals_and_flags() {
    let args = parse_bump_args(&["a.json", "b.json", "-r", "--output", "OUT.md"]);
    assert_eq!(args.files, vec!["a.json".to_string(), "b.json".to_string()]);
    assert!(args.recursive);
    assert_eq!(args.output, "OUT.md");
  }

  #[test]
  fn bump_defaults() {
    let args = parse_bump_args(&[]);
    assert!(args.files.is_empty());
    assert!(!args.recursive);
    assert_eq!(args.output, "CHANGELOG.md");
  }

  #[test]
  fn bump_output_equals_forms() {
    assert_eq!(parse_bump_args(&["--output=OUT.md"]).output, "OUT.md");
    assert_eq!(parse_bump_args(&["-o=OUT.md"]).output, "OUT.md");
    assert_eq!(parse_bump_args(&["-oOUT.md"]).output, "OUT.md");
  }

  #[test]
  fn bump_short_flag_cluster() {
    let args = parse_bump_args(&["-ro", "OUT.md"]);
    assert!(args.recursive);
    assert_eq!(args.output, "OUT.md");
  }

  #[test]
  fn bump_double_dash_treats_rest_as_files() {
    let args = parse_bump_args(&["--", "-r", "--output"]);
    assert_eq!(args.files, vec!["-r".to_string(), "--output".to_string()]);
    assert!(!args.recursive);
  }

  #[test]
  fn bump_unknown_flags_error() {
    assert_eq!(
      parse(&argv(&["--wat"])).unwrap_err(),
      "unknown option: --wat".to_string()
    );
    assert_eq!(
      parse(&argv(&["-x"])).unwrap_err(),
      "unknown option: -x".to_string()
    );
  }

  #[test]
  fn bump_missing_output_value_errors() {
    assert_eq!(
      parse(&argv(&["--output"])).unwrap_err(),
      "option --output requires a value".to_string()
    );
    assert_eq!(
      parse(&argv(&["-o"])).unwrap_err(),
      "option -o requires a value".to_string()
    );
  }

  #[test]
  fn overrides_omit_empty_files() {
    // ADR-0013 浅合并语义锚定：空 files 不注入键
    let overrides = bump_overrides(&BumpArgs {
      files: vec![],
      recursive: false,
      output: "CHANGELOG.md".to_string(),
    });
    assert!(!overrides.contains_key("files"));
    assert_eq!(overrides["recursive"], json!(false));
    assert_eq!(overrides["changelog"], json!({ "output": "CHANGELOG.md" }));
  }

  #[test]
  fn overrides_include_nonempty_files() {
    let overrides = bump_overrides(&BumpArgs {
      files: vec!["a.json".to_string()],
      recursive: true,
      output: "OUT.md".to_string(),
    });
    assert_eq!(overrides["files"], json!(["a.json"]));
    assert_eq!(overrides["recursive"], json!(true));
    assert_eq!(overrides["changelog"], json!({ "output": "OUT.md" }));
  }
}
