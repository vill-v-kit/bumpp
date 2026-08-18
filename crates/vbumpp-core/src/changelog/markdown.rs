//! markdown 生成：结构逐节对齐 changelogen 0.6.2 `generateMarkDown`。
//! 申报偏差两处：无 ungh.cc 网络解析、scope 级排除内建默认（chore 组
//! `deps`，经 `types.X.excludeScopes` 可配置——changelog.rs 管线）；
//! 原申报偏差①（中文节标题直生）随英文默认移除。

use crate::changelog::config::ChangelogConfig;
use crate::changelog::gitmoji::convert_gitmoji;
use crate::commits::{CommitReference, DisplayCommit, ReferenceType};
use crate::git::RepoConfig;

/// 版本区间（运行时入参——`from` / `to` / `newVersion` 不进配置文件）
#[derive(Debug, Clone, Copy)]
pub struct ReleaseRange<'a> {
  pub from: &'a str,
  pub to: &'a str,
  pub new_version: Option<&'a str>,
}

/// changelogen `generateMarkDown`：节序 = `## ` 头 → compare 链接 → 类型分组
/// （声明序、空组跳过、组内 reverse）→ breaking 节 → 贡献者节 → gitmoji 转换
pub fn generate_markdown(
  commits: &[DisplayCommit],
  config: &ChangelogConfig,
  range: &ReleaseRange,
) -> String {
  // 上游 `config.newVersion && ...`：空串按 falsy 回落 from...to
  let v = range
    .new_version
    .filter(|nv| !nv.is_empty())
    .map(|nv| config.tag_body.replace("{{newVersion}}", nv));
  let mut markdown: Vec<String> = vec![
    String::new(),
    format!(
      "## {}",
      v.clone()
        .unwrap_or_else(|| format!("{}...{}", range.from, range.to))
    ),
    String::new(),
  ];
  // changelogen：`config.repo && config.from` 时输出 compare 链接
  if let Some(repo) = &config.repo {
    if !range.from.is_empty() {
      markdown.push(format_compare_changes(v.as_deref(), repo, range));
    }
  }
  let mut breaking_lines: Vec<String> = vec![];
  for (type_name, entry) in &config.types {
    let group: Vec<&DisplayCommit> = commits
      .iter()
      .filter(|c| &c.commit_type == type_name)
      .collect();
    if group.is_empty() {
      continue;
    }
    markdown.push(String::new());
    markdown.push(format!("### {}", entry.title));
    markdown.push(String::new());
    // changelogen：group.reverse()（git log 新→旧，组内翻转为旧→新）
    for commit in group.iter().rev() {
      let line = format_commit(commit, config.repo.as_ref());
      if commit.is_breaking {
        breaking_lines.push(line.clone());
      }
      markdown.push(line);
    }
  }
  if !breaking_lines.is_empty() {
    // 节标题直生（原申报偏差①的正当形态保留，标题语言随英文默认）；
    // 回退链对齐原 hack：BreakingChange 非对象（被 false 禁用）时回落英文默认
    let title = config
      .types
      .iter()
      .find(|(n, _)| n == "BreakingChange")
      .map(|(_, e)| e.title.clone())
      .unwrap_or_else(|| "⚠️ Breaking Changes".to_owned());
    markdown.push(String::new());
    markdown.push(format!("#### {title}"));
    markdown.push(String::new());
    markdown.extend(breaking_lines);
  }
  let authors = collect_authors(commits, config);
  // 贡献者节头英文（原申报偏差①已随英文默认移除）；无 ungh.cc 解析（申报偏差②）
  if !authors.is_empty() && !config.no_authors {
    markdown.push(String::new());
    markdown.push("### ❤️ Contributors".to_owned());
    markdown.push(String::new());
    for (name, emails) in authors {
      markdown.push(format!("- {name}{}", author_email_part(&emails, config)));
    }
  }
  convert_gitmoji(markdown.join("\n").trim())
}

/// changelogen `formatCommit`：`- **scope:** ⚠️  Description (refs)`
fn format_commit(commit: &DisplayCommit, repo: Option<&RepoConfig>) -> String {
  let scope_part = if commit.scope.is_empty() {
    String::new()
  } else {
    format!("**{}:** ", commit.scope.trim())
  };
  let breaking_part = if commit.is_breaking { "⚠️  " } else { "" };
  format!(
    "- {scope_part}{breaking_part}{}{}",
    upper_first(&commit.description),
    format_references(&commit.references, repo)
  )
}

/// changelogen `formatReferences`：PR + issue 全部列出（PR 先）；否则取首个
/// （恒为 hash 引用）；空则无
fn format_references(references: &[CommitReference], repo: Option<&RepoConfig>) -> String {
  let pr = references
    .iter()
    .filter(|r| r.ref_type == ReferenceType::PullRequest);
  let issue = references
    .iter()
    .filter(|r| r.ref_type == ReferenceType::Issue);
  let listed: Vec<String> = pr.chain(issue).map(|r| format_reference(r, repo)).collect();
  if !listed.is_empty() {
    return format!(" ({})", listed.join(", "));
  }
  if let Some(first) = references.first() {
    return format!(" ({})", format_reference(first, repo));
  }
  String::new()
}

/// `https://{domain}/{repo}`（changelogen `baseUrl`）
fn repo_base_url(repo: &RepoConfig) -> String {
  let domain = repo.domain.as_deref().unwrap_or("");
  let name = repo.repo.as_deref().unwrap_or("");
  format!("https://{domain}/{name}")
}

/// changelogen `formatReference`：provider 非 github / gitlab / bitbucket
/// （含 repo 缺失）返回原文，不出链接
fn format_reference(reference: &CommitReference, repo: Option<&RepoConfig>) -> String {
  let Some(repo) = repo else {
    return reference.value.clone();
  };
  let path = match (repo.provider.as_deref(), reference.ref_type) {
    (Some("github"), ReferenceType::PullRequest) => "pull",
    (Some("github"), ReferenceType::Issue) => "issues",
    (Some("github"), ReferenceType::Hash) => "commit",
    (Some("gitlab"), ReferenceType::PullRequest) => "merge_requests",
    (Some("gitlab"), ReferenceType::Issue) => "issues",
    (Some("gitlab"), ReferenceType::Hash) => "commit",
    (Some("bitbucket"), ReferenceType::PullRequest) => "pull-requests",
    (Some("bitbucket"), ReferenceType::Issue) => "issues",
    (Some("bitbucket"), ReferenceType::Hash) => "commit",
    _ => return reference.value.clone(),
  };
  format!(
    "[{}]({}/{path}/{})",
    reference.value,
    repo_base_url(repo),
    reference.value.trim_start_matches('#')
  )
}

/// changelogen `formatCompareChanges`：**任意 provider 恒出链接**（仅 bitbucket
/// 走 `branches/compare/{to}%0D{from}` 特判，其余 `compare/{from}...{to}`）；
/// `to` 优先取 tagBody 渲染值
fn format_compare_changes(v: Option<&str>, repo: &RepoConfig, range: &ReleaseRange) -> String {
  let to = v.unwrap_or(range.to);
  let (part, changes) = if repo.provider.as_deref() == Some("bitbucket") {
    ("branches/compare", format!("{to}%0D{}", range.from))
  } else {
    ("compare", format!("{}...{to}", range.from))
  };
  format!(
    "[compare changes]({}/{part}/{changes})",
    repo_base_url(repo)
  )
}

/// changelogen `formatName`：按空格分段，逐段 upperFirst
fn format_name(name: &str) -> String {
  name
    .split(' ')
    .map(|p| upper_first(p.trim()))
    .collect::<Vec<_>>()
    .join(" ")
}

/// 贡献者收集（changelogen 同名逻辑）：formatName 规范化、空名与 `[bot]` 跳过、
/// excludeAuthors 子串匹配（name 或 email）、按名去重并汇集邮箱（保序）
fn collect_authors(
  commits: &[DisplayCommit],
  config: &ChangelogConfig,
) -> Vec<(String, Vec<String>)> {
  let mut authors: Vec<(String, Vec<String>)> = vec![];
  for commit in commits {
    let name = format_name(&commit.author.name);
    if name.is_empty() || name.contains("[bot]") {
      continue;
    }
    let email = &commit.author.email;
    if config
      .exclude_authors
      .iter()
      .any(|ex| name.contains(ex) || email.contains(ex))
    {
      continue;
    }
    match authors.iter_mut().find(|(n, _)| *n == name) {
      Some((_, emails)) => {
        if !emails.contains(email) {
          emails.push(email.clone());
        }
      }
      None => authors.push((name, vec![email.clone()])),
    }
  }
  authors
}

/// 贡献者行的邮箱段：hideAuthorEmail 时为空（默认）；否则取首个
/// 非 noreply.github.com 邮箱 ` <email>`，无则为空
fn author_email_part(emails: &[String], config: &ChangelogConfig) -> String {
  if config.hide_author_email {
    return String::new();
  }
  emails
    .iter()
    .find(|e| !e.contains("noreply.github.com"))
    .map(|e| format!(" <{e}>"))
    .unwrap_or_default()
}

/// scule `upperFirst`：首字符大写，其余原样
fn upper_first(s: &str) -> String {
  let mut chars = s.chars();
  match chars.next() {
    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    None => String::new(),
  }
}
