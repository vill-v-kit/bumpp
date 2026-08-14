//! CLI 应用层（ADR-0016）：argv 语法（子命令、flag、help 文案、错误提示、
//! 退出码）的唯一归属。手写解析器（语法盘子小，不引 clap）；npm bin 与原生
//! CLI 二进制（`crates/vbumpp`，ADR-0016）为共享 `run_from_argv` 的两个
//! 薄壳前端。
//!
//! 消息文案逐条对齐 JS 时代 cli.ts 的 consola 输出（parity 基准）；着色仿
//! `progress.rs` 先例（dialoguer `console::style`，非 TTY 自动降级纯文本）。

use std::io::Write;
use std::path::{Path, PathBuf};

use dialoguer::console::style;
use serde_json::{json, Map, Value};

use crate::token;

/// 真实入口：npm bin / 原生 bin 的唯一调用点。`provider` 为平台变体注入身份
/// （bump 与 release 通路生效；token 子命令无视）——argv `--provider` flag
/// 优先于该注入（ADR-0016）。返回退出码，由调用壳回写
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
/// `VBUMPP_TOKEN_STORE` → `VBUMPP_HOME` → 系统 home，ADR-0013）；`cwd` 覆盖
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
    Ok(Command::Bump(args)) => bump_command(&args, provider, env, out, err),
    Ok(Command::Release(args)) => release_command(&args, provider, env, out, err),
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
  Release(ReleaseArgs),
}

/// bump 默认命令（`vbumpp [...files]`）的解析产物
struct BumpArgs {
  files: Vec<String>,
  recursive: bool,
  output: String,
  provider: Option<String>,
  dry_run: bool,
}

/// release 子命令（`vbumpp release <version>`，ADR-0016）的解析产物
struct ReleaseArgs {
  version: String,
  output: String,
  provider: Option<String>,
  dry_run: bool,
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

// ---------------------------------------------------------------------------
// bump 默认命令
// ---------------------------------------------------------------------------

/// argv → overrides（旧 cli.ts 的 JS 对象构造原样收编）：`recursive` 与
/// `changelog.output` 始终传（cac 默认值语义）；`files` 仅在非空时注入——
/// ADR-0013 浅合并语义，空 files 整体替换掉配置文件的 files 是旧 defu 行为。
/// dry-run 注入 confirm=false——`Bump?` 确认在预览语义下跳过（零写盘无需
/// 二次确认），经配置浅合并在流水线内生效，流水线零预览分支
fn bump_overrides(args: &BumpArgs) -> Map<String, Value> {
  let mut overrides = Map::new();
  if !args.files.is_empty() {
    overrides.insert("files".to_string(), json!(args.files));
  }
  overrides.insert("recursive".to_string(), json!(args.recursive));
  overrides.insert("changelog".to_string(), json!({ "output": args.output }));
  if args.dry_run {
    overrides.insert("confirm".to_string(), json!(false));
  }
  overrides
}

/// provider 解析（ADR-0016）：argv `--provider` flag 优先于平台变体注入身份；
/// 两者皆无为 None（bump 后不接 release；release 子命令在执行层判必填）
fn resolve_provider(
  flag: Option<&str>,
  injected: Option<&str>,
) -> Result<Option<crate::release::Provider>, String> {
  flag
    .or(injected)
    .map(|p| {
      crate::release::Provider::parse(p).ok_or_else(|| {
        format!("unknown provider: {p} (expected github / gitlab / gitee / gitcode)")
      })
    })
    .transpose()
}

fn bump_command(
  args: &BumpArgs,
  provider: Option<&str>,
  env: &RunEnv,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  let provider = match resolve_provider(args.provider.as_deref(), provider) {
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
  // dry-run：全链只读计算照走（校验失败照常 exit 1），打印执行计划
  if args.dry_run {
    return bump_dry_run(&options, &cwd, out, err);
  }
  match crate::orchestrate::bump_version(&options, &cwd) {
    Ok(_) => 0,
    Err(e) => {
      error_line(err, &e.to_string());
      1
    }
  }
}

// ---------------------------------------------------------------------------
// bump dry-run（COL-85）：计划装配骑完整编排（预演与执行同路），此处只负责
// 渲染——开头标识 dry run（全程无 success 行）、逐文件预演判定、版本与来源、
// 将写盘清单、脚本与命令文本、git 动作完整文本、changelog 全文预览、
// --provider 时的平台 Release 预览（COL-84 渲染同形）
// ---------------------------------------------------------------------------

fn bump_dry_run(
  options: &crate::orchestrate::BumpVersionOptions,
  cwd: &Path,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  match crate::bump_plan::plan_bump(options, cwd) {
    Ok(plan) => {
      info_line(out, "bump plan (dry run — no changes made)");
      // 逐文件预演判定（与真实执行同一代码段产出的三态）
      for (file, verdict) in &plan.verdicts {
        let line = match verdict {
          crate::plugins::FileVerdict::Updated => {
            format!("{file}: update → {}", plan.new_version)
          }
          crate::plugins::FileVerdict::UpToDate => format!("{file}: up-to-date"),
          crate::plugins::FileVerdict::Missing => format!("{file}: missing"),
        };
        info_line(out, &line);
      }
      info_line(
        out,
        &format!(
          "current version: {} (source: {})",
          plan.current_version, plan.current_version_source
        ),
      );
      info_line(out, &format!("new version: {}", plan.new_version));
      if !plan.writes.is_empty() {
        info_line(out, "files to write:");
        for path in &plan.writes {
          info_line(out, &format!("  {}", crate::display::path(cwd, path)));
        }
      }
      if !plan.scripts.is_empty() || !plan.installs.is_empty() || plan.execute.is_some() {
        info_line(out, "commands to run:");
      }
      for (slot, command) in &plan.scripts {
        info_line(out, &format!("  {slot}: {command}"));
      }
      for install in &plan.installs {
        info_line(out, &format!("  install: {install}"));
      }
      if let Some(execute) = &plan.execute {
        info_line(out, &format!("  execute: {execute}"));
      }
      if plan.commit_message.is_some() || plan.tag_name.is_some() || !plan.pushes.is_empty() {
        info_line(out, "git actions:");
      }
      if let Some(message) = &plan.commit_message {
        info_line(out, &format!("  commit: {message}"));
      }
      if let Some(tag) = &plan.tag_name {
        info_line(out, &format!("  tag: {tag}"));
      }
      for push in &plan.pushes {
        info_line(out, &format!("  {push}"));
      }
      match &plan.changelog {
        Some(markdown) => {
          info_line(out, "changelog preview:");
          let _ = writeln!(out, "{markdown}");
        }
        None => info_line(out, "changelog: skipped (no previous git tag)"),
      }
      // --provider 组合：平台 Release 预览（COL-84 渲染同形）
      if let Some(release) = &plan.release {
        print_release_plan(release, out);
      }
      0
    }
    Err(e) => {
      error_line(err, &e.to_string());
      1
    }
  }
}

/// bump dry-run 的平台 Release 计划行渲染（与 `release_dry_run` 同一份行格式）
fn print_release_plan(plan: &crate::release::ReleasePlan, out: &mut impl Write) {
  info_line(out, "release plan (dry run — no changes made)");
  match &plan.token_source {
    Some(source) => info_line(out, &format!("token source: {}", source.describe())),
    // 警告行复用真实执行的报错文案（仅降级不改动措辞，同一事实源）
    None => warn_line(out, &crate::release::missing_token_message(plan.provider)),
  }
  info_line(out, &format!("provider: {}", plan.provider.display()));
  info_line(out, &format!("host: {}", plan.host));
  info_line(out, &format!("repo: {}/{}", plan.owner, plan.repo));
  info_line(out, &format!("tag_name: {}", plan.tag_name));
  info_line(out, &format!("prerelease: {}", plan.prerelease));
  info_line(out, "body:");
  let _ = writeln!(out, "{}", plan.body);
  info_line(out, "requests:");
  for request in &plan.requests {
    info_line(out, &format!("  {} {}", request.method, request.url));
  }
}

// ---------------------------------------------------------------------------
// release 子命令（ADR-0016）：bump 末段 release 失败（网络 / 密钥过期）后的
// 独立重试通路——body 从 changelog 文件提取指定版本节，纯创建语义
// ---------------------------------------------------------------------------

fn release_command(
  args: &ReleaseArgs,
  provider: Option<&str>,
  env: &RunEnv,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  let provider = match resolve_provider(args.provider.as_deref(), provider) {
    Ok(Some(provider)) => provider,
    Ok(None) => {
      error_line(
        err,
        "option --provider is required for release (expected github / gitlab / gitee / gitcode)",
      );
      return 1;
    }
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
  // 版本归一化：`5.1.0` / `v5.1.0` 均接受，内部一律裸版本号
  let version = args.version.strip_prefix('v').unwrap_or(&args.version);
  let tag = format!("v{version}");

  // 前置校验①：本地 tag 必须存在——github-like 请求体的 tag_name 在 tag 缺失
  // 时会让平台 API 在默认分支 HEAD 静默建 tag（危险的错误发布）
  let tag_ref = format!("refs/tags/{tag}");
  if crate::exec::capture(
    "git",
    &[
      "rev-parse".into(),
      "--verify".into(),
      "--quiet".into(),
      tag_ref,
    ],
    &cwd,
  )
  .is_err()
  {
    error_line(
      err,
      &format!(
        "tag {tag} not found locally — run the bump flow first (release requires an existing tag)"
      ),
    );
    return 1;
  }

  // 前置校验②：changelog 中必须存在该版本节——防静默发空 body 的错误 release
  let changelog_path = cwd.join(&args.output);
  let content = match std::fs::read_to_string(&changelog_path) {
    Ok(content) => content,
    Err(e) => {
      error_line(err, &format!("cannot read {}: {e}", args.output));
      return 1;
    }
  };
  let Some(markdown) = crate::changelog::extract_version_section(&content, version) else {
    error_line(
      err,
      &format!("no changelog section found for {tag} in {}", args.output),
    );
    return 1;
  };

  // dry-run：校验已全走（上方任一失败照常 exit 1，可当 CI 预检门禁），
  // 此处起拦截全部平台 HTTP，打印执行计划
  if args.dry_run {
    return release_dry_run(provider, version, &markdown, &cwd, out, err);
  }

  match crate::release::create_release(provider, version, &markdown, &cwd, None) {
    Ok(()) => {
      success_line(
        out,
        &format!("[{}] add release {tag} success", provider.display()),
      );
      0
    }
    Err(e) => {
      error_line(err, &e.to_string());
      1
    }
  }
}

/// release dry-run（COL-84）：计划装配骑真实创建链（预演与执行同路），
/// 渲染与 bump dry-run 的 release 块共用 `print_release_plan`
fn release_dry_run(
  provider: crate::release::Provider,
  version: &str,
  markdown: &str,
  cwd: &Path,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  match crate::release::plan_release(provider, version, markdown, cwd, None) {
    Ok(plan) => {
      print_release_plan(&plan, out);
      0
    }
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
      let Some(scanned) = scan_token_flags_or_report(&args[1..], err) else {
        return 1;
      };
      let Some(name) = scanned.positionals.first() else {
        error_line(err, "usage: vbumpp token set <name>");
        return 1;
      };
      // host 作用域键虽 provider 无关，但目前只有 gitlab 的 release 解析链
      // 有 host 配置通路——其他 provider 存进去永远不会被读到，一律拒绝
      // （未来 GHE 支持时解除）
      let key = match scanned.values.get("host") {
        Some(raw_host) => {
          if name != "gitlab" {
            error_line(
              err,
              &format!("--host is only supported for gitlab (got {name})"),
            );
            return 1;
          }
          match token::host_scoped_key(name, raw_host) {
            Ok(key) => key,
            Err(e) => {
              error_line(err, &e.to_string());
              return 1;
            }
          }
        }
        None => name.clone(),
      };
      // 友好显示形态：provider 级键 `gitlab`，host 作用域键
      // `gitlab (https://gitlab-a.com)`——prompt 文案指明目标 host
      let display = display_key(&key);
      // 密码交互在 Rust 侧（dialoguer，ADR-0014）
      let prompt = env.prompt.unwrap_or(&token::prompt_token);
      match prompt(&display) {
        Ok(Some(plaintext)) => {
          let Some(store) = resolve_store(env.store, err) else {
            return 1;
          };
          match token::save_token_at(&store, &key, &plaintext) {
            Ok(()) => {
              success_line(out, &format!("{display} token saved (encrypted)"));
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
      let Some(scanned) = scan_token_flags_or_report(&args[1..], err) else {
        return 1;
      };
      // `--host` 过滤单条：过滤值与写入侧经同一规范化函数相撞
      let host_filter = match scanned.values.get("host") {
        Some(raw_host) => match token::normalize_host(raw_host) {
          Ok(host) => Some(host),
          Err(e) => {
            error_line(err, &e.to_string());
            return 1;
          }
        },
        None => None,
      };
      let Some(store) = resolve_store(env.store, err) else {
        return 1;
      };
      match token::read_token_store_at(&store) {
        Ok(tokens) if tokens.is_empty() => {
          info_line(out, "no tokens configured");
          0
        }
        Ok(tokens) => {
          let mut shown = 0;
          for key in tokens.keys() {
            if let Some(filter) = &host_filter {
              if !matches!(token::split_key(key), (_, Some(host)) if host == filter) {
                continue;
              }
            }
            info_line(out, &display_key(key));
            shown += 1;
          }
          if shown == 0 {
            let filter = host_filter.expect("only reachable with a filter");
            warn_line(out, &format!("no token found for host {filter}"));
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

/// token 子命令 flag 扫描产物
#[derive(Debug)]
struct TokenArgs {
  /// 位置参数（`--` 分隔之后一律按位置参数收集，dash 前缀不再解析）
  positionals: Vec<String>,
  /// 布尔 flag 命中集（bare `--flag`；为 remove 矩阵 --all/--yes/--dry-run 打底）
  #[allow(dead_code)]
  flags: std::collections::BTreeSet<String>,
  /// 值 flag 命中表（`--flag value` 与 `--flag=value` 双形态；重复取最后）
  values: std::collections::BTreeMap<String, String>,
}

/// token 子命令的 flag 扫描小 helper：认 `--flag` / `--flag=value` / `--`
/// 位置参数分隔；声明名单外的 `--x` 与短 flag 一律未知报错（exit 1 由调用方
/// 回写）。手写解析维持，不引 clap（ADR-0016）
fn scan_token_args(
  args: &[String],
  bool_flags: &[&str],
  value_flags: &[&str],
) -> Result<TokenArgs, String> {
  let mut scanned = TokenArgs {
    positionals: Vec::new(),
    flags: std::collections::BTreeSet::new(),
    values: std::collections::BTreeMap::new(),
  };
  let mut positional_only = false;
  let mut i = 0;
  while i < args.len() {
    let arg = &args[i];
    i += 1;
    if positional_only {
      scanned.positionals.push(arg.clone());
      continue;
    }
    if arg == "--" {
      positional_only = true;
      continue;
    }
    if let Some(long) = arg.strip_prefix("--") {
      let (name, inline) = match long.split_once('=') {
        Some((name, value)) => (name, Some(value.to_string())),
        None => (long, None),
      };
      if value_flags.contains(&name) {
        let value = match inline {
          Some(value) => value,
          None => {
            let Some(next) = args.get(i) else {
              return Err(format!("option --{name} requires a value"));
            };
            // flag 形态的下一参数不当值吞（`--host --foo` 必为漏值笔误，
            // 吞下去会落出 `gitlab@https://--foo` 这类静默错键）
            if next.starts_with('-') && next.len() > 1 {
              return Err(format!("option --{name} requires a value"));
            }
            i += 1;
            next.clone()
          }
        };
        scanned.values.insert(name.to_string(), value);
      } else if bool_flags.contains(&name) {
        // 布尔 flag 带值按 mri 惯例视为 truthy（与 bump/release 解析一致）
        scanned.flags.insert(name.to_string());
      } else {
        return Err(format!("unknown option: {arg}"));
      }
      continue;
    }
    if arg.starts_with('-') && arg.len() > 1 {
      // token 子命令无短 flag——dash 前缀不当位置参数吞（原 cac parity 的
      // 「name 缺省即用法错误」升级为显式未知 flag 报错）
      return Err(format!("unknown option: {arg}"));
    }
    scanned.positionals.push(arg.clone());
  }
  Ok(scanned)
}

/// 列表 / prompt 的友好显示形态：`gitlab` / `gitlab (https://gitlab-a.com)`
fn display_key(key: &str) -> String {
  match token::split_key(key) {
    (provider, Some(host)) => format!("{provider} ({host})"),
    _ => key.to_owned(),
  }
}

/// set / list 当前共用的 flag 名单（remove 矩阵落地时另带 --all/--yes/--dry-run）
const TOKEN_VALUE_FLAGS: &[&str] = &["host"];

/// token 子命令扫描的收口：解析失败即报错回写、调用方 `return 1`
fn scan_token_flags_or_report(args: &[String], err: &mut impl Write) -> Option<TokenArgs> {
  match scan_token_args(args, &[], TOKEN_VALUE_FLAGS) {
    Ok(scanned) => Some(scanned),
    Err(message) => {
      error_line(err, &message);
      None
    }
  }
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
     vbumpp release <version>         retry platform release from a changelog section\n  \
     vbumpp token <action> [name]     manage tokens (action: set / list / remove), stored encrypted\n  \
     (set/list accept --host <url> for gitlab self-hosted instances)\n\
     \noptions:\n  \
     -o, --output [output]       where CHANGELOG.md is generated / read (default CHANGELOG.md)\n  \
     -r, --recursive             recursively\n  \
     --provider <provider>       release provider (github / gitlab / gitee / gitcode)\n  \
     --dry-run                   preview the bump/release plan without side effects\n  \
     -h, --help                  show help\n  \
     -v, --version               show version"
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
        Self::Release(args) => write!(f, "Release(version={:?})", args.version),
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
  fn scan_token_args_value_flag_both_forms() {
    let scanned = scan_token_args(&argv(&["gitlab", "--host", "a.com"]), &[], &["host"]).unwrap();
    assert_eq!(scanned.positionals, vec!["gitlab".to_string()]);
    assert_eq!(
      scanned.values.get("host").map(String::as_str),
      Some("a.com")
    );

    let scanned = scan_token_args(&argv(&["gitlab", "--host=a.com"]), &[], &["host"]).unwrap();
    assert_eq!(
      scanned.values.get("host").map(String::as_str),
      Some("a.com")
    );
  }

  #[test]
  fn scan_token_args_double_dash_stops_flag_parsing() {
    let scanned = scan_token_args(&argv(&["--", "--host", "a.com"]), &[], &["host"]).unwrap();
    assert_eq!(
      scanned.positionals,
      vec!["--host".to_string(), "a.com".to_string()],
      "-- 之后一律位置参数"
    );
    assert!(scanned.values.is_empty());
  }

  #[test]
  fn scan_token_args_unknown_flag_and_missing_value_error() {
    let err = scan_token_args(&argv(&["--wat"]), &[], &["host"]).unwrap_err();
    assert_eq!(err, "unknown option: --wat");
    let err = scan_token_args(&argv(&["-x"]), &[], &["host"]).unwrap_err();
    assert_eq!(err, "unknown option: -x");
    let err = scan_token_args(&argv(&["--host"]), &[], &["host"]).unwrap_err();
    assert_eq!(err, "option --host requires a value");
  }

  #[test]
  fn scan_token_args_value_flag_rejects_flag_shaped_value() {
    // `--host --foo` 必为漏值笔误——flag 形态的下一参数不当值吞
    let err = scan_token_args(&argv(&["--host", "--foo"]), &[], &["host"]).unwrap_err();
    assert_eq!(err, "option --host requires a value");
    let err = scan_token_args(&argv(&["--host", "-x"]), &[], &["host"]).unwrap_err();
    assert_eq!(err, "option --host requires a value");
  }

  #[test]
  fn scan_token_args_bool_flag_truthy_with_value() {
    // mri 惯例：布尔 flag 带值亦视为命中（与 bump/release 解析一致）
    let scanned = scan_token_args(&argv(&["--all=false"]), &["all"], &[]).unwrap();
    assert!(scanned.flags.contains("all"));
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
      provider: None,
      dry_run: false,
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
      provider: None,
      dry_run: false,
    });
    assert_eq!(overrides["files"], json!(["a.json"]));
    assert_eq!(overrides["recursive"], json!(true));
    assert_eq!(overrides["changelog"], json!({ "output": "OUT.md" }));
  }

  #[test]
  fn bump_provider_flag_forms() {
    assert_eq!(
      parse_bump_args(&["--provider", "gitee"])
        .provider
        .as_deref(),
      Some("gitee")
    );
    assert_eq!(
      parse_bump_args(&["--provider=github"]).provider.as_deref(),
      Some("github")
    );
    assert_eq!(parse_bump_args(&[]).provider, None);
    assert_eq!(
      parse(&argv(&["--provider"])).unwrap_err(),
      "option --provider requires a value".to_string()
    );
  }

  #[test]
  fn provider_flag_beats_injection() {
    // ADR-0016 优先级锚定：argv flag > 平台变体注入
    let resolved = resolve_provider(Some("gitee"), Some("github")).unwrap();
    assert_eq!(resolved, Some(crate::release::Provider::Gitee));
    let resolved = resolve_provider(None, Some("github")).unwrap();
    assert_eq!(resolved, Some(crate::release::Provider::Github));
    assert_eq!(resolve_provider(None, None).unwrap(), None);
    assert!(resolve_provider(Some("wat"), None).is_err());
  }

  #[test]
  fn release_parses_version_and_flags() {
    match parse(&argv(&[
      "release",
      "5.1.0",
      "--provider",
      "gitee",
      "-o",
      "OUT.md",
    ])) {
      Ok(Command::Release(args)) => {
        assert_eq!(args.version, "5.1.0");
        assert_eq!(args.provider.as_deref(), Some("gitee"));
        assert_eq!(args.output, "OUT.md");
      }
      other => panic!("应为 Release，实际 {other:?}"),
    }
    match parse(&argv(&["release", "v5.1.0", "--provider=gitlab"])) {
      Ok(Command::Release(args)) => {
        assert_eq!(args.version, "v5.1.0");
        assert_eq!(args.provider.as_deref(), Some("gitlab"));
        assert_eq!(args.output, "CHANGELOG.md");
      }
      other => panic!("应为 Release，实际 {other:?}"),
    }
  }

  #[test]
  fn release_requires_version() {
    assert_eq!(
      parse(&argv(&["release"])).unwrap_err(),
      "usage: vbumpp release <version> --provider <github|gitlab|gitee|gitcode>".to_string()
    );
    assert_eq!(
      parse(&argv(&["release", "--provider", "gitee"])).unwrap_err(),
      "usage: vbumpp release <version> --provider <github|gitlab|gitee|gitcode>".to_string()
    );
  }

  #[test]
  fn release_rejects_extra_positionals_and_unknown_flags() {
    assert_eq!(
      parse(&argv(&["release", "1.0.0", "2.0.0"])).unwrap_err(),
      "unexpected argument: 2.0.0".to_string()
    );
    assert_eq!(
      parse(&argv(&["release", "1.0.0", "--wat"])).unwrap_err(),
      "unknown option: --wat".to_string()
    );
    assert_eq!(
      parse(&argv(&["release", "1.0.0", "-r"])).unwrap_err(),
      "unknown option: -r".to_string()
    );
  }

  #[test]
  fn bump_parses_dry_run_flag() {
    // 无值 flag：缺省 false；`--dry-run` 与 `=值` 形态（mri truthy 惯例）均开启
    assert!(!parse_bump_args(&[]).dry_run);
    assert!(parse_bump_args(&["--dry-run"]).dry_run);
    assert!(parse_bump_args(&["--dry-run=false"]).dry_run);
    assert!(parse_bump_args(&["a.json", "--dry-run"]).dry_run);
  }

  #[test]
  fn dry_run_overrides_disable_confirm() {
    // Bump? 确认在 dry-run 跳过（零写盘无需二次确认）：经 overrides 注入
    // confirm=false，流水线零预览分支
    let overrides = bump_overrides(&BumpArgs {
      files: vec![],
      recursive: false,
      output: "CHANGELOG.md".to_string(),
      provider: None,
      dry_run: true,
    });
    assert_eq!(overrides["confirm"], json!(false));
    // 非 dry-run 不注入（交互语义与配置四层合并不受影响）
    let overrides = bump_overrides(&BumpArgs {
      files: vec![],
      recursive: false,
      output: "CHANGELOG.md".to_string(),
      provider: None,
      dry_run: false,
    });
    assert!(!overrides.contains_key("confirm"));
  }

  #[test]
  fn release_parses_dry_run_flag() {
    // 无值 flag：缺省 false；`--dry-run` 与 `=值` 形态（mri truthy 惯例）均开启
    match parse(&argv(&["release", "1.0.0"])) {
      Ok(Command::Release(args)) => assert!(!args.dry_run),
      other => panic!("应为 Release，实际 {other:?}"),
    }
    for forms in [
      &["release", "1.0.0", "--dry-run"][..],
      &["release", "1.0.0", "--dry-run=false"][..],
      &["release", "--dry-run", "1.0.0"][..],
    ] {
      match parse(&argv(forms)) {
        Ok(Command::Release(args)) => assert!(args.dry_run, "{forms:?}"),
        other => panic!("应为 Release，实际 {other:?}"),
      }
    }
  }
}
