//! changelog 域（ADR-0012）：changelogen 使用面的 Rust 重写。
//! 编排与对外 API 收于此根部；能力子目录：配置段解析（`config`）、
//! markdown 生成（`markdown`）、gitmoji 数据表（`gitmoji`）。

pub mod config;
pub mod gitmoji;
pub mod markdown;

use std::error::Error;
use std::fmt;
use std::path::Path;

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

use crate::commits::{parse_display_commit, DisplayCommit};
use crate::git::RawCommit;

use crate::changelog::config::{resolve_changelog_config, ChangelogConfig, ChangelogConfigError};
use crate::changelog::markdown::ReleaseRange;

/// 引擎管线：RawCommit → 展示层解析 → 类型过滤（config.types 键）→
/// `chore(deps)` 过滤（chore + scope deps + 非 breaking；scope 为 scopeMap
/// 应用后的值——与原 JS 同一位点的 quirk 保持一致）→ markdown 生成。
/// 全程纯函数、无 IO、无网络。
pub fn render_changelog(
  raw_commits: &[RawCommit],
  config: &ChangelogConfig,
  range: &ReleaseRange,
) -> String {
  let commits: Vec<DisplayCommit> = raw_commits
    .iter()
    .filter_map(|raw| parse_display_commit(raw, &config.scope_map))
    .filter(|c| config.types.iter().any(|(n, _)| n == &c.commit_type))
    .filter(|c| !(c.commit_type == "chore" && c.scope == "deps" && !c.is_breaking))
    .collect();
  markdown::generate_markdown(&commits, config, range)
}

// ---------------------------------------------------------------------------
// generateChangelog 编排（替代原 JS changelog.ts 全部职责，ADR-0012 修复清单）
// ---------------------------------------------------------------------------

/// `generateChangelog` 入参：`overrides` 为扁平全量配置覆盖（bumpp 键 +
/// `changelog` 键，与 `.vbumpprc.json` 同形）；`from` 为真实 tag 名
/// （`getLastGitTag` 结果，C1）；`to` 为新版本号
#[derive(Debug, Clone, Default)]
pub struct GenerateChangelogOptions {
  pub overrides: Option<Map<String, Value>>,
  pub from: String,
  pub to: String,
}

/// `generateChangelog` 返回（对齐原 JS `ChangelogResult`，附带解析后的输出路径供编排打印）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateChangelogOutcome {
  /// 当前发版的 markdown 信息
  pub markdown: String,
  /// changelog 文件的全部信息
  pub changelog_md: String,
  /// 解析后的输出文件路径（配置 `changelog.output`，编排的进度打印取此而非重推）
  pub output: String,
}

#[derive(Debug)]
pub enum ChangelogError {
  Config { message: String },
  Schema { message: String },
  Git { message: String },
  Io { message: String },
}

impl fmt::Display for ChangelogError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Config { message }
      | Self::Schema { message }
      | Self::Git { message }
      | Self::Io { message } => f.write_str(message),
    }
  }
}

impl Error for ChangelogError {}

impl From<crate::config::LoadConfigError> for ChangelogError {
  fn from(e: crate::config::LoadConfigError) -> Self {
    Self::Config {
      message: e.to_string(),
    }
  }
}

impl From<ChangelogConfigError> for ChangelogError {
  fn from(e: ChangelogConfigError) -> Self {
    Self::Schema {
      message: e.to_string(),
    }
  }
}

impl From<crate::exec::ExecError> for ChangelogError {
  fn from(e: crate::exec::ExecError) -> Self {
    Self::Git {
      message: e.to_string(),
    }
  }
}

/// 既有条目定位（原 JS `/^###?\s+.*$/m` 首个匹配）
static FIRST_ENTRY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^###?\s+.*$").unwrap());

/// `generateChangelog` 编排：统一配置解析（`.vbumpprc.json` + overrides 透传 +
/// 内建默认，单一解析路径）→ `getGitDiff` → 引擎 → 读既有 CHANGELOG 插入/追加 →
/// 写盘 → 按 bumpp `commit` 开关提交（C2）；全程无网络（ADR-0012）
pub fn generate_changelog(
  options: &GenerateChangelogOptions,
  cwd: &Path,
) -> Result<GenerateChangelogOutcome, ChangelogError> {
  // 统一配置解析：同一份文档一次读取——bumpp 键（含 commit 开关）经
  // merge_bump_config，changelog 段经 resolve_changelog_config（ADR-0013）
  let document = crate::config::read_document(
    cwd,
    crate::config::custom_config_path(options.overrides.as_ref()).as_deref(),
  )?;
  let merged = crate::config::merge_bump_config(options.overrides.clone(), document.clone());
  let section_overrides = options
    .overrides
    .as_ref()
    .and_then(|o| o.get("changelog"))
    .and_then(Value::as_object);
  let mut config = resolve_changelog_config(document.as_ref(), section_overrides)?;
  // changelogen `config.repo ||= resolveRepoConfig(cwd)`：配置未覆盖时自
  // package.json `repository` / git remote 解析
  if config.repo.is_none() {
    config.repo = crate::git::resolve_repo_config(cwd);
  }

  let raw_commits = crate::git::get_git_diff(cwd, &options.from, None)?;
  let range = ReleaseRange {
    from: &options.from,
    to: &options.to,
    new_version: Some(&options.to),
  };
  let markdown = render_changelog(&raw_commits, &config, &range);

  let output = cwd.join(&config.output);
  let changelog_md = upsert_changelog(&output, &markdown)?;

  // C2：提交跟随统一配置的 bumpp commit 开关（JS truthiness：`false` / `""` 为关，
  // 字符串即上游自定义提交信息形态、视为开启）；N1：仅 add 实际写出的 output 文件；
  // C3：commitMessage 模板（{{output}} 占位）——对齐原 JS 的无 flag 调用形态
  let commit_enabled = match merged.get("commit") {
    None => true, // 内建默认 true
    Some(Value::Bool(b)) => *b,
    Some(Value::String(s)) => !s.is_empty(),
    Some(_) => true,
  };
  if commit_enabled {
    let message = config.commit_message.replace("{{output}}", &config.output);
    crate::exec::run("git", &["add".into(), config.output.clone()], cwd)?;
    crate::exec::run("git", &["commit".into(), "-m".into(), message], cwd)?;
  }

  Ok(GenerateChangelogOutcome {
    markdown,
    changelog_md,
    output: config.output,
  })
}

/// 读既有文件（缺失则以 `# Changelog\n\n` 起始）→ 首个 `^###?` 条目前插入
/// （无则追加）→ 写盘；返回最终全文
fn upsert_changelog(output: &Path, markdown: &str) -> Result<String, ChangelogError> {
  let mut changelog_md = match std::fs::read_to_string(output) {
    Ok(content) => content,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => "# Changelog\n\n".to_owned(),
    Err(e) => {
      return Err(ChangelogError::Io {
        message: format!("读取 {} 失败：{e}", output.display()),
      })
    }
  };
  changelog_md = match FIRST_ENTRY_RE.find(&changelog_md) {
    Some(m) => format!(
      "{}{markdown}\n\n{}",
      &changelog_md[..m.start()],
      &changelog_md[m.start()..]
    ),
    None => format!("{changelog_md}\n{markdown}\n\n"),
  };
  std::fs::write(output, &changelog_md).map_err(|e| ChangelogError::Io {
    message: format!("写入 {} 失败：{e}", output.display()),
  })?;
  Ok(changelog_md)
}
