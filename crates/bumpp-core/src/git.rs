//! git 操作：shell out 到 `git` 二进制（继承用户 git config / SSH / GPG / credential helper），
//! 对齐上游 bumpp v11 `gitCommit` / `gitTag` / `gitPush`。

use std::path::Path;

use crate::exec::{run, ExecError};
use crate::progress::ProgressEvent;

/// git commit 的输入参数（对齐上游 options.commit + operation.state）
pub struct CommitSpec<'a> {
  /// 已更新的文件（`all: false` 时按路径逐个提交）
  pub updated_files: &'a [String],
  pub all: bool,
  pub no_verify: bool,
  pub sign: bool,
  /// commit 信息模板（`%s` 替换为新版本号，无占位符则追加版本号）
  pub message: &'a str,
  pub new_version: &'a str,
}

/// git tag 的输入参数（对齐上游 options.tag + options.commit.message）
pub struct TagSpec<'a> {
  /// tag 名模板（同样支持 `%s`）
  pub name: &'a str,
  /// 附注信息模板（上游复用 commit.message）
  pub message: &'a str,
  pub sign: bool,
  pub new_version: &'a str,
}

/// 上游 `formatVersionString`：含 `%s` 则全部替换，否则在末尾追加版本号
pub fn format_version_string(template: &str, new_version: &str) -> String {
  if template.contains("%s") {
    template.replace("%s", new_version)
  } else {
    format!("{template}{new_version}")
  }
}

/// 上游 `gitCommit`：`--allow-empty [--all] [--no-verify] [--gpg-sign] --message <msg> [files...]`
pub fn git_commit(cwd: &Path, spec: &CommitSpec) -> Result<(ProgressEvent, String), ExecError> {
  let mut args = vec!["commit".to_string(), "--allow-empty".to_string()];
  if spec.all {
    args.push("--all".to_string());
  }
  if spec.no_verify {
    args.push("--no-verify".to_string());
  }
  if spec.sign {
    args.push("--gpg-sign".to_string());
  }
  let message = format_version_string(spec.message, spec.new_version);
  args.push("--message".to_string());
  args.push(message.clone());
  if !spec.all {
    args.extend(spec.updated_files.iter().cloned());
  }
  run("git", &args, cwd)?;
  Ok((ProgressEvent::GitCommit, message))
}

/// 上游 `gitTag`：`--annotate --message <commit.message 格式化> <tagName> [--sign]`
/// 注意：git tag 没有 hooks，上游不加 --no-verify
pub fn git_tag(cwd: &Path, spec: &TagSpec) -> Result<(ProgressEvent, String), ExecError> {
  let tag_name = format_version_string(spec.name, spec.new_version);
  let mut args = vec![
    "tag".to_string(),
    "--annotate".to_string(),
    "--message".to_string(),
    format_version_string(spec.message, spec.new_version),
    tag_name.clone(),
  ];
  if spec.sign {
    args.push("--sign".to_string());
  }
  run("git", &args, cwd)?;
  Ok((ProgressEvent::GitTag, tag_name))
}

/// 上游 `gitPush`：`git push`，启用 tag 时追加 `git push --tags`
pub fn git_push(cwd: &Path, with_tags: bool) -> Result<ProgressEvent, ExecError> {
  run("git", &["push".to_string()], cwd)?;
  if with_tags {
    run("git", &["push".to_string(), "--tags".to_string()], cwd)?;
  }
  Ok(ProgressEvent::GitPush)
}
