//! 手写 argv 解析器（不引 clap——语法盘子小；未来子命令家族长大再迁，只动
//! cli 模块）：解析产物（Command / BumpArgs / ReleaseArgs）与三路解析入口
//! （parse / parse_bump / parse_release）。

/// 解析结果：全局 flag 优先于子命令判定（cac parity——`--help` 出现即显示帮助，
/// 无视其他参数）；`--` 之后一律视为位置参数。
#[derive(Debug)]
pub enum Command {
  Help,
  Version,
  Token(Vec<String>),
  Bump(BumpArgs),
  Release(ReleaseArgs),
}

/// bump 默认命令（`vbumpp [...files]`）的解析产物
#[derive(Debug)]
pub struct BumpArgs {
  pub files: Vec<String>,
  pub recursive: bool,
  pub output: String,
  pub provider: Option<String>,
  pub dry_run: bool,
}

/// release 子命令（`vbumpp release <version>`，ADR-0016）的解析产物
#[derive(Debug)]
pub struct ReleaseArgs {
  pub version: String,
  pub output: String,
  pub provider: Option<String>,
  pub dry_run: bool,
}

pub fn parse(argv: &[String]) -> Result<Command, String> {
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
    Some("release") => parse_release(&argv[1..]),
    _ => parse_bump(argv),
  }
}

fn parse_bump(argv: &[String]) -> Result<Command, String> {
  let mut args = BumpArgs {
    files: Vec::new(),
    recursive: false,
    output: "CHANGELOG.md".to_string(),
    provider: None,
    dry_run: false,
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
      "--dry-run" => args.dry_run = true,
      "-o" | "--output" => {
        let Some(value) = argv.get(i) else {
          return Err(format!("option {arg} requires a value"));
        };
        args.output = value.clone();
        i += 1;
      }
      "--provider" => {
        let Some(value) = argv.get(i) else {
          return Err("option --provider requires a value".to_string());
        };
        args.provider = Some(value.clone());
        i += 1;
      }
      _ if arg.starts_with("--") => match arg.split_once('=') {
        Some(("--output", value)) => args.output = value.to_string(),
        Some(("--provider", value)) => args.provider = Some(value.to_string()),
        // 布尔 flag 带值按 mri 惯例视为 truthy（--recursive=false 与
        // --dry-run=false 亦开启）
        Some(("--recursive", _)) => args.recursive = true,
        Some(("--dry-run", _)) => args.dry_run = true,
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

/// release 子命令解析（ADR-0016）：`vbumpp release <version> [-o file]
/// [--provider name]`。version 为唯一位置参数（多余位置参数即用法错误——
/// 与 bump 的 files 列表语义不同）；`--provider` 的必填判定在命令执行层
/// （平台变体注入可兜底，解析层不知注入身份）。
fn parse_release(argv: &[String]) -> Result<Command, String> {
  let mut version = None;
  let mut output = "CHANGELOG.md".to_string();
  let mut provider = None;
  let mut dry_run = false;
  let mut positional_only = false;
  let mut i = 0;
  while i < argv.len() {
    let arg = &argv[i];
    i += 1;
    if positional_only {
      if version.replace(arg.clone()).is_some() {
        return Err(format!("unexpected argument: {arg}"));
      }
      continue;
    }
    match arg.as_str() {
      "--" => positional_only = true,
      "--dry-run" => dry_run = true,
      "-o" | "--output" => {
        let Some(value) = argv.get(i) else {
          return Err(format!("option {arg} requires a value"));
        };
        output = value.clone();
        i += 1;
      }
      "--provider" => {
        let Some(value) = argv.get(i) else {
          return Err("option --provider requires a value".to_string());
        };
        provider = Some(value.clone());
        i += 1;
      }
      _ if arg.starts_with("--") => match arg.split_once('=') {
        Some(("--output", value)) => output = value.to_string(),
        Some(("--provider", value)) => provider = Some(value.to_string()),
        // 布尔 flag 带值按 mri 惯例视为 truthy（--dry-run=false 亦开启）
        Some(("--dry-run", _)) => dry_run = true,
        _ => return Err(format!("unknown option: {arg}")),
      },
      _ if arg.starts_with('-') && arg.len() > 1 => {
        // 仅 -o 有短形态（与 bump 对齐）；release 无布尔短 flag
        let cluster = &arg[1..];
        let Some(rest) = cluster.strip_prefix('o') else {
          return Err(format!("unknown option: -{}", &cluster[..1]));
        };
        let rest = rest.strip_prefix('=').unwrap_or(rest);
        if !rest.is_empty() {
          output = rest.to_string();
        } else {
          let Some(value) = argv.get(i) else {
            return Err("option -o requires a value".to_string());
          };
          output = value.clone();
          i += 1;
        }
      }
      _ => {
        if version.replace(arg.clone()).is_some() {
          return Err(format!("unexpected argument: {arg}"));
        }
      }
    }
  }
  let Some(version) = version else {
    return Err(
      "usage: vbumpp release <version> --provider <github|gitlab|gitee|gitcode>".to_string(),
    );
  };
  Ok(Command::Release(ReleaseArgs {
    version,
    output,
    provider,
    dry_run,
  }))
}
