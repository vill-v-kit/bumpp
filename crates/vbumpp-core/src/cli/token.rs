//! token 子命令（消息文案与 cli.ts switch 逐条 parity）：set / list / remove
//! 三动作 + flag 扫描小 helper（ADR-0035 remove 交互矩阵所在地）。

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::output::{error_line, info_line, success_line, warn_line};
use super::RunEnv;
use crate::token;

pub(super) fn token_command(
  args: &[String],
  env: &RunEnv,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  let Some(action) = args.first() else {
    error_line(
      err,
      "usage: vbumpp token <action> [name] (action: set / list / remove)",
    );
    return 1;
  };
  match action.as_str() {
    "set" => {
      let Some(scanned) = scan_token_flags_or_report(&args[1..], &[], err) else {
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
          let Some(key) = gitlab_host_scoped_key_or_report(name, raw_host, err) else {
            return 1;
          };
          key
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
      let Some(scanned) = scan_token_flags_or_report(&args[1..], &[], err) else {
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
      let Some(scanned) = scan_token_flags_or_report(&args[1..], REMOVE_BOOL_FLAGS, err) else {
        return 1;
      };
      // --host 与 --all 语义互斥（精确单键 vs 全量），同给即用法错误
      if scanned.flags.contains("all") && scanned.values.contains_key("host") {
        error_line(err, "options --host and --all cannot be used together");
        return 1;
      }
      let name = scanned.positionals.first();
      if name.is_none() && !scanned.flags.contains("all") {
        error_line(err, "usage: vbumpp token remove <name>");
        return 1;
      }
      let Some(store) = resolve_store(env.store, err) else {
        return 1;
      };
      let tokens = match token::read_token_store_at(&store) {
        Ok(tokens) => tokens,
        Err(e) => {
          error_line(err, &e.to_string());
          return 1;
        }
      };
      // 目标解析（四形态；目标不存在 / 全无匹配沿用 warn + exit 0）
      let targets: Vec<&String> = if scanned.flags.contains("all") {
        match name {
          // provider --all：provider 级键 + 全部 host 作用域键（键格式归属
          // token.rs——split_key 的 provider 段一致即匹配，含旧式不透明键）
          Some(name) => tokens
            .keys()
            .filter(|key| token::split_key(key).0 == name.as_str())
            .collect(),
          // 全量 --all：清空所有 provider
          None => tokens.keys().collect(),
        }
      } else if let Some(raw_host) = scanned.values.get("host") {
        let name = name.expect("name checked above");
        let Some(key) = gitlab_host_scoped_key_or_report(name, raw_host, err) else {
          return 1;
        };
        if !tokens.contains_key(&key) {
          warn_line(out, &format!("no token found for {}", display_key(&key)));
          return 0;
        }
        // 精确项语义：不触碰同 provider 的其他键
        tokens.keys().filter(|k| k.as_str() == key).collect()
      } else {
        let name = name.expect("name checked above");
        if !tokens.contains_key(name) {
          warn_line(out, &format!("no token found for {name}"));
          return 0;
        }
        tokens.keys().filter(|key| key.as_str() == name).collect()
      };
      if targets.is_empty() {
        match name {
          Some(name) => warn_line(out, &format!("no token found for {name}")),
          None => warn_line(out, "no tokens configured"),
        }
        return 0;
      }
      // --dry-run 优先级最高：只打印将删清单，不确认、不删除（与 --yes 同给亦然）
      let dry_run = scanned.flags.contains("dry-run");
      if dry_run {
        info_line(out, "tokens to remove (dry run — no changes made):");
      } else {
        info_line(out, "tokens to remove:");
      }
      for key in &targets {
        info_line(out, &format!("  {}", display_key(key)));
      }
      if dry_run {
        return 0;
      }
      // 二次确认（--yes 跳过）：dialoguer Confirm 默认 No，拒绝即取消 exit 0；
      // 非 TTY 无法交互由真实实现报错引导 --yes（不能静默删）
      if !scanned.flags.contains("yes") {
        let confirm = env.confirm.unwrap_or(&token::confirm_remove);
        match confirm("Remove the listed tokens?") {
          Ok(true) => {}
          Ok(false) => {
            warn_line(out, "canceled");
            return 0;
          }
          Err(e) => {
            error_line(err, &e.to_string());
            return 1;
          }
        }
      }
      for key in &targets {
        match token::remove_token_at(&store, key) {
          Ok(_) => success_line(out, &format!("{} token removed", display_key(key))),
          Err(e) => {
            error_line(err, &e.to_string());
            return 1;
          }
        }
      }
      0
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

/// token 子命令 flag 扫描产物
#[derive(Debug)]
pub struct TokenArgs {
  /// 位置参数（`--` 分隔之后一律按位置参数收集，dash 前缀不再解析）
  pub positionals: Vec<String>,
  /// 布尔 flag 命中集（bare `--flag`；remove 矩阵 --all/--yes/--dry-run 消费）
  pub flags: BTreeSet<String>,
  /// 值 flag 命中表（`--flag value` 与 `--flag=value` 双形态；重复取最后）
  pub values: BTreeMap<String, String>,
}

/// token 子命令的 flag 扫描小 helper：认 `--flag` / `--flag=value` / `--`
/// 位置参数分隔；声明名单外的 `--x` 与短 flag 一律未知报错（exit 1 由调用方
/// 回写）。手写解析维持，不引 clap（ADR-0016）
pub fn scan_token_args(
  args: &[String],
  bool_flags: &[&str],
  value_flags: &[&str],
) -> Result<TokenArgs, String> {
  let mut scanned = TokenArgs {
    positionals: Vec::new(),
    flags: BTreeSet::new(),
    values: BTreeMap::new(),
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

/// set / list / remove 共用的值 flag 名单
const TOKEN_VALUE_FLAGS: &[&str] = &["host"];

/// remove 矩阵的布尔 flag 名单（set / list 无布尔 flag）
const REMOVE_BOOL_FLAGS: &[&str] = &["all", "yes", "dry-run"];

/// `--host` 的 gitlab 门禁 + 复合键构建（set / remove 共用）：非 gitlab 拒绝、
/// host 无效报错，失败均已回写 err（调用方 `return 1`）
fn gitlab_host_scoped_key_or_report(
  name: &str,
  raw_host: &str,
  err: &mut impl Write,
) -> Option<String> {
  if name != "gitlab" {
    error_line(
      err,
      &format!("--host is only supported for gitlab (got {name})"),
    );
    return None;
  }
  match token::host_scoped_key(name, raw_host) {
    Ok(key) => Some(key),
    Err(e) => {
      error_line(err, &e.to_string());
      None
    }
  }
}

/// token 子命令扫描的收口：解析失败即报错回写、调用方 `return 1`
fn scan_token_flags_or_report(
  args: &[String],
  bool_flags: &[&str],
  err: &mut impl Write,
) -> Option<TokenArgs> {
  match scan_token_args(args, bool_flags, TOKEN_VALUE_FLAGS) {
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
