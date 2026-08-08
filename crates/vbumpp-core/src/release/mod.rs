//! 平台 Release（ADR-0014）：github / gitlab / gitee / gitcode 的 release 创建。
//! 布局（ADR-0014）：每 provider 单文件——github / gitee / gitcode 为薄文件
//! （各持 base_url 与 token 注入形态：Bearer 头 / 请求体字段 / query 参数），
//! 请求体语义的单一事实源在 github_like.rs；gitlab.rs 全特化（`PRIVATE-TOKEN`
//! 头 + 项目 id 直查 + `gitlab.host` 解析随文件，自建实例严格 schema 仅 host 一键）。
//!
//! 本文件持：Provider 词汇（跨模块：CLI 入参、napi 边界、token 存储键）、
//! ReleaseError、token 解析链与 create_release 分发；四家共用的仓库信息解析与
//! HTTP 原语在 http.rs。
//!
//! token 解析链统一为：Token 存储 → 各家环境变量 →（仅 github）`gh auth token`
//! 兜底。明文 token 不出本模块（ADR-0014：不跨 napi 边界）——错误消息经
//! `ReleaseError::redact` 脱敏（原始与 form 编码形态都替换为掩码）。

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

use serde_json::{Map, Value};

use crate::git::RepoConfig;

pub mod gitcode;
pub mod gitee;
pub mod github;
mod github_like;
pub mod gitlab;
mod http;

/// 四家托管平台
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
  Github,
  Gitlab,
  Gitee,
  Gitcode,
}

impl Provider {
  /// napi / CLI 入参的字符串形态
  pub fn parse(s: &str) -> Option<Self> {
    match s {
      "github" => Some(Self::Github),
      "gitlab" => Some(Self::Gitlab),
      "gitee" => Some(Self::Gitee),
      "gitcode" => Some(Self::Gitcode),
      _ => None,
    }
  }

  /// token 存储与环境变量后缀的键名
  pub fn name(self) -> &'static str {
    match self {
      Self::Github => "github",
      Self::Gitlab => "gitlab",
      Self::Gitee => "gitee",
      Self::Gitcode => "gitcode",
    }
  }

  /// 进度与报错文案中的展示名（对齐 JS 时代 spinner 文案）
  pub fn display(self) -> &'static str {
    match self {
      Self::Github => "Github",
      Self::Gitlab => "Gitlab",
      Self::Gitee => "Gitee",
      Self::Gitcode => "GitCode",
    }
  }

  /// 环境变量回退链（ADR-0014：github 为 GH_TOKEN → GITHUB_TOKEN——拼错的
  /// GITHOB_TOKEN 已随重写移除；其余三家补 CI 场景缺失的环境变量通道）
  fn env_vars(self) -> &'static [&'static str] {
    match self {
      Self::Github => &["GH_TOKEN", "GITHUB_TOKEN"],
      Self::Gitlab => &["GITLAB_TOKEN"],
      Self::Gitee => &["GITEE_TOKEN"],
      Self::Gitcode => &["GITCODE_TOKEN"],
    }
  }
}

#[derive(Debug)]
pub enum ReleaseError {
  Token { message: String },
  Config { message: String },
  Git { message: String },
  Http { message: String },
}

impl fmt::Display for ReleaseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Token { message }
      | Self::Config { message }
      | Self::Git { message }
      | Self::Http { message } => f.write_str(message),
    }
  }
}

impl Error for ReleaseError {}

impl ReleaseError {
  /// 报错脱敏（ADR-0014："明文 token 不出本模块"对错误消息同样成立）：
  /// token 的**原始形态与 form 编码形态**都替换为掩码——gitcode 经 query
  /// 注入 token，ureq 传输报错可能含完整 URL；服务端错误回显（check_status
  /// 提取的 message / 原始响应体）亦属不可控输入，两家形态都可能出现
  fn redact(self, token: &str) -> Self {
    if token.is_empty() {
      return self;
    }
    const MASK: &str = "[redacted]";
    let encoded: String = url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
    let scrub = |message: String| message.replace(token, MASK).replace(&encoded, MASK);
    match self {
      Self::Token { message } => Self::Token {
        message: scrub(message),
      },
      Self::Config { message } => Self::Config {
        message: scrub(message),
      },
      Self::Git { message } => Self::Git {
        message: scrub(message),
      },
      Self::Http { message } => Self::Http {
        message: scrub(message),
      },
    }
  }
}

impl From<crate::exec::ExecError> for ReleaseError {
  fn from(e: crate::exec::ExecError) -> Self {
    Self::Git {
      message: e.to_string(),
    }
  }
}

impl From<crate::config::LoadConfigError> for ReleaseError {
  fn from(e: crate::config::LoadConfigError) -> Self {
    Self::Config {
      message: e.to_string(),
    }
  }
}

/// 创建平台 release：token 解析 → 仓库信息 → 各家 API。`overrides` 为编排传入的
/// 程序化覆盖（`gitlab.host` 与 `configFilePath` 经此参与解析）
pub fn create_release(
  provider: Provider,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
  overrides: Option<&Map<String, Value>>,
) -> Result<(), ReleaseError> {
  let token = resolve_token_real(provider, cwd)?;
  // 信息行（对齐 JS resolveRepoConfig 的 consola.info）
  if let Some(RepoConfig {
    domain,
    repo: Some(_),
    ..
  }) = crate::git::resolve_repo_config(cwd)
  {
    println!(
      "{} repo: domain {} ({provider})",
      dialoguer::console::style("ℹ").blue(),
      domain.unwrap_or_else(|| "unknown".into()),
      provider = provider.display(),
    );
  }
  match provider {
    Provider::Github => github::create(&token, new_version, markdown, cwd),
    Provider::Gitee => gitee::create(&token, new_version, markdown, cwd),
    Provider::Gitcode => gitcode::create(&token, new_version, markdown, cwd),
    Provider::Gitlab => gitlab::create(&token, new_version, markdown, cwd, overrides),
  }
}

// ---------------------------------------------------------------------------
// token 解析链
// ---------------------------------------------------------------------------

/// 解析链纯核（ADR-0014）：store → 各家环境变量 →（仅 github）gh CLI。
/// 三个数据源注入以便测试；空字符串按 JS `||` 语义视为缺失
#[doc(hidden)]
pub fn resolve_token(
  provider: Provider,
  tokens: &BTreeMap<String, String>,
  env: &dyn Fn(&str) -> Option<String>,
  gh_cli: &dyn Fn() -> Option<String>,
) -> Option<String> {
  if let Some(token) = tokens.get(provider.name()) {
    if !token.is_empty() {
      return Some(token.clone());
    }
  }
  for var in provider.env_vars() {
    if let Some(value) = env(var) {
      if !value.is_empty() {
        return Some(value);
      }
    }
  }
  if provider == Provider::Github {
    return gh_cli().and_then(|t| {
      let trimmed = t.trim().to_owned();
      (!trimmed.is_empty()).then_some(trimmed)
    });
  }
  None
}

/// 生产数据源包装：真实 token 存储（读取失败警告后按空表继续，对齐 JS
/// bump.ts 的 catch-warn 语义）→ `std::env::var` → `gh auth token`
fn resolve_token_real(provider: Provider, cwd: &Path) -> Result<String, ReleaseError> {
  let tokens = match crate::token::read_token_store() {
    Ok(tokens) => tokens,
    Err(e) => {
      eprintln!(
        "{} failed to read token store file ({e}); run vbumpp token set <name> again",
        dialoguer::console::style("⚠").yellow(),
      );
      BTreeMap::new()
    }
  };
  resolve_token(provider, &tokens, &|key| std::env::var(key).ok(), &|| {
    crate::exec::capture("gh", &["auth".into(), "token".into()], cwd)
      .ok()
      .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
  })
  .ok_or_else(|| ReleaseError::Token {
    message: format!(
      "no {} token detected; run vbumpp token set {} to add one",
      provider.display(),
      provider.name()
    ),
  })
}
