//! release 子命令：bump 末段 release 失败（网络 / 密钥过期）后的
//! 独立重试通路——body 从 changelog 文件提取指定版本节，纯创建语义；
//! changelog 路径 `-o` 优先、未给时回落配置 `changelog.output`//! 与 bump 同路解析）；dry-run计划装配骑真实创建链，渲染复用
//! bump 模块的 `print_release_plan`。

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use super::bump::{print_release_plan, resolve_provider};
use super::output::{error_line, success_line};
use super::parse::ReleaseArgs;
use super::RunEnv;
use crate::changelog::config::resolve_changelog_config;
use crate::changelog::extract_version_section;
use crate::config::read_document;
use crate::exec::capture;
use crate::release::{create_release, plan_release, Provider};

/// changelog 文件路径解析：`-o` 未给出时经与 bump 同路的配置
/// 解析取 `changelog.output`（全局 ← 项目文档合并 + changelog 段解析），
/// 内建默认 CHANGELOG.md 兜底；配置不可解析即时报错（与 bump 通路一致，
/// 不得静默回落默认）
fn resolve_changelog_path(cwd: &Path) -> Result<String, String> {
  let document = read_document(cwd, None).map_err(|e| e.to_string())?;
  resolve_changelog_config(document.as_ref(), None)
    .map(|config| config.output)
    .map_err(|e| e.to_string())
}

pub(super) fn release_command(
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
    None => match env::current_dir() {
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
  if capture(
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
  // changelog 路径：flag 优先，未给时回落配置 changelog.output
  let changelog_file = match &args.output {
    Some(path) => path.clone(),
    None => match resolve_changelog_path(&cwd) {
      Ok(path) => path,
      Err(message) => {
        error_line(err, &message);
        return 1;
      }
    },
  };
  let changelog_path = cwd.join(&changelog_file);
  let content = match fs::read_to_string(&changelog_path) {
    Ok(content) => content,
    Err(e) => {
      error_line(err, &format!("cannot read {changelog_file}: {e}"));
      return 1;
    }
  };
  let Some(markdown) = extract_version_section(&content, version) else {
    error_line(
      err,
      &format!("no changelog section found for {tag} in {changelog_file}"),
    );
    return 1;
  };

  // dry-run：校验已全走（上方任一失败照常 exit 1，可当 CI 预检门禁），
  // 此处起拦截全部平台 HTTP，打印执行计划
  if args.dry_run {
    return release_dry_run(provider, version, &markdown, &cwd, out, err);
  }

  match create_release(provider, version, &markdown, &cwd, None) {
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

/// release dry-run：计划装配骑真实创建链（预演与执行同路），
/// 渲染与 bump dry-run 的 release 块共用 `print_release_plan`
fn release_dry_run(
  provider: Provider,
  version: &str,
  markdown: &str,
  cwd: &Path,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  match plan_release(provider, version, markdown, cwd, None) {
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
