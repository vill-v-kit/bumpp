//! bump 默认命令：argv → overrides 构造与 provider 解析、真实执行，以及
//! dry-run 计划渲染（COL-85）——平台 Release 计划行的渲染（print_release_plan）
//! 在此，release 子命令的 dry-run 复用。

use std::io::Write;
use std::path::Path;

use serde_json::{json, Map, Value};

use super::output::{error_line, info_line, warn_line};
use super::parse::BumpArgs;
use super::RunEnv;

/// argv → overrides（旧 cli.ts 的 JS 对象构造原样收编）：`recursive` 与
/// `changelog.output` 始终传（cac 默认值语义）；`files` 仅在非空时注入——
/// ADR-0013 浅合并语义，空 files 整体替换掉配置文件的 files 是旧 defu 行为。
/// dry-run 注入 confirm=false——`Bump?` 确认在预览语义下跳过（零写盘无需
/// 二次确认），经配置浅合并在流水线内生效，流水线零预览分支
pub fn bump_overrides(args: &BumpArgs) -> Map<String, Value> {
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
pub fn resolve_provider(
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

pub(super) fn bump_command(
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

/// bump dry-run（COL-85）：计划装配骑完整编排（预演与执行同路），此处只负责
/// 渲染——开头标识 dry run（全程无 success 行）、逐文件预演判定、版本与来源、
/// 将写盘清单、脚本与命令文本、git 动作完整文本、changelog 全文预览、
/// --provider 时的平台 Release 预览（COL-84 渲染同形）
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
pub(super) fn print_release_plan(plan: &crate::release::ReleasePlan, out: &mut impl Write) {
  info_line(out, "release plan (dry run — no changes made)");
  match &plan.token_source {
    Some(source) => info_line(out, &format!("token source: {}", source.describe())),
    // 警告行复用真实执行的报错文案（仅降级不改动措辞，同一事实源）；
    // plan.host 即有效 host（gitlab 缺失文案的 --host 指引消费）
    None => warn_line(
      out,
      &crate::release::missing_token_message(plan.provider, Some(&plan.host)),
    ),
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
