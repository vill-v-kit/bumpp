//! 平台 Release（ADR-0014）：github / gitlab / gitee / gitcode 的 release 创建。
//! 共享 github-like 实现为 Rust 内部细节（gitee 请求体带 token、gitcode query 带
//! token、github Bearer 头）；gitlab 特化——`PRIVATE-TOKEN` 头 + 项目 id 直查
//! （`GET /api/v4/projects/<url编码的 owner/repo>`，替代 JS 时代的搜索 + 后缀
//! 匹配两步法），自建实例经配置 `gitlab.host` 段（严格 schema：仅 host 一键）。
//!
//! token 解析链统一为：Token 存储 → 各家环境变量 →（仅 github）`gh auth token`
//! 兜底。明文 token 不出本模块（ADR-0014：不跨 napi 边界）。

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::LazyLock;

use serde_json::{json, Map, Value};

use crate::git::RepoConfig;

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
      "{} repo: domain {}（{provider}）",
      dialoguer::console::style("ℹ").blue(),
      domain.unwrap_or_else(|| "unknown".into()),
      provider = provider.display(),
    );
  }
  match provider {
    Provider::Github => create_github_like_release(
      provider,
      "https://api.github.com",
      &token,
      new_version,
      markdown,
      cwd,
    ),
    Provider::Gitee => create_github_like_release(
      provider,
      "https://gitee.com/api/v5",
      &token,
      new_version,
      markdown,
      cwd,
    ),
    Provider::Gitcode => create_github_like_release(
      provider,
      "https://api.gitcode.com/api/v5",
      &token,
      new_version,
      markdown,
      cwd,
    ),
    Provider::Gitlab => {
      let document =
        crate::config::read_document(cwd, crate::config::custom_config_path(overrides).as_deref())?;
      let host = resolve_gitlab_host(document.as_ref(), overrides)?
        .unwrap_or_else(|| "https://gitlab.com".to_owned());
      create_gitlab_release(&host, &token, new_version, markdown, cwd)
    }
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
        "{} token 存储文件读取失败（{e}），请重新执行 vbumpp token set <name>",
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
      "未检测到 {} token，请执行 vbumpp token set {} 录入",
      provider.display(),
      provider.name()
    ),
  })
}

// ---------------------------------------------------------------------------
// gitlab 配置段（严格 schema：仅 host 一键，ADR-0014）
// ---------------------------------------------------------------------------

/// `gitlab.host` 解析：四层语义——overrides 段 > 文件段（文件段已含全局←项目合并）
#[doc(hidden)]
pub fn resolve_gitlab_host(
  document: Option<&Map<String, Value>>,
  overrides: Option<&Map<String, Value>>,
) -> Result<Option<String>, ReleaseError> {
  // 校验与提取收归 config::gitlab_host_of（文件层在 read_config 已先验过一次）
  let mut host = None;
  for source in [document, overrides].into_iter().flatten() {
    if let Some(h) = crate::config::gitlab_host_of(source)? {
      host = Some(h);
    }
  }
  Ok(host)
}

// ---------------------------------------------------------------------------
// 仓库信息与 HTTP 原语
// ---------------------------------------------------------------------------

/// `owner/repo` 解析（package.json `repository` 优先、git remote 兜底）
fn resolve_owner_repo(cwd: &Path) -> Result<(String, String), ReleaseError> {
  let repo = crate::git::resolve_repo_config(cwd)
    .and_then(|r| r.repo)
    .ok_or_else(|| ReleaseError::Git {
      message: "无法获取远程仓库信息".into(),
    })?;
  repo
    .split_once('/')
    .map(|(o, r)| (o.to_owned(), r.to_owned()))
    .ok_or_else(|| ReleaseError::Git {
      message: "无法获取远程仓库信息".into(),
    })
}

/// 共享 Agent：30s 全局超时；状态码不当异常（手动检查以提取服务端错误信息）
fn agent() -> ureq::Agent {
  ureq::Agent::config_builder()
    .timeout_global(Some(std::time::Duration::from_secs(30)))
    .http_status_as_error(false)
    .build()
    .into()
}

/// 非 2xx 报错：提取服务端 `message` 字段（gitlab/gitee 错误体形态），
/// 无法解析时回落原始响应体
fn check_status(
  resp: &mut ureq::http::Response<ureq::Body>,
  provider: Provider,
) -> Result<(), ReleaseError> {
  let status = resp.status().as_u16();
  if (200..300).contains(&status) {
    return Ok(());
  }
  let body = resp.body_mut().read_to_string().unwrap_or_default();
  let server_message = serde_json::from_str::<Value>(&body)
    .ok()
    .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_owned))
    .filter(|m| !m.is_empty())
    .unwrap_or_else(|| {
      if body.is_empty() {
        "Unknown error".into()
      } else {
        body
      }
    });
  Err(ReleaseError::Http {
    message: format!(
      "{} [open api] error : [{status}] {server_message}",
      provider.display()
    ),
  })
}

fn post_json(
  agent: &ureq::Agent,
  url: &str,
  headers: &[(&str, String)],
  body: &Value,
  provider: Provider,
) -> Result<(), ReleaseError> {
  let mut request = agent.post(url);
  for (name, value) in headers {
    request = request.header(*name, value);
  }
  let mut resp = request.send_json(body).map_err(|e| ReleaseError::Http {
    message: format!("{} [open api] error : {e}", provider.display()),
  })?;
  check_status(&mut resp, provider)
}

// ---------------------------------------------------------------------------
// github-like（github / gitee / gitcode 共享实现，差异仅在 token 注入方式）
// ---------------------------------------------------------------------------

static PRERELEASE_RE: LazyLock<regex::Regex> =
  LazyLock::new(|| regex::Regex::new(r"(beta|alpha)").unwrap());

/// base_url 可注入（测试指向本地 mock）；生产经 `create_release` 传各家真实地址
#[doc(hidden)]
pub fn create_github_like_release(
  provider: Provider,
  base_url: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  debug_assert!(matches!(
    provider,
    Provider::Github | Provider::Gitee | Provider::Gitcode
  ));
  let (owner, repo) = resolve_owner_repo(cwd)?;
  let branch = crate::git::get_current_git_branch(cwd)?;
  let mut body = json!({
    "name": new_version,
    "tag_name": format!("v{new_version}"),
    "body": markdown,
    "target_commitish": branch,
    "prerelease": PRERELEASE_RE.is_match(new_version),
  });
  let mut url = format!("{base_url}/repos/{owner}/{repo}/releases");
  let mut headers: Vec<(&str, String)> = vec![];
  match provider {
    Provider::Github => {
      headers.push(("x-github-api-version", "2022-11-28".to_owned()));
      headers.push(("authorization", format!("Bearer {token}")));
    }
    Provider::Gitee => {
      body["access_token"] = Value::String(token.to_owned());
    }
    Provider::Gitcode => {
      let encoded: String = url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
      url = format!("{url}?access_token={encoded}");
    }
    Provider::Gitlab => unreachable!("gitlab 不走 github-like 通道"),
  }
  post_json(&agent(), &url, &headers, &body, provider)
}

// ---------------------------------------------------------------------------
// gitlab（PRIVATE-TOKEN + 项目 id 直查）
// ---------------------------------------------------------------------------

/// host 可注入（测试指向本地 mock）；生产经 `create_release` 解析 `gitlab.host`
#[doc(hidden)]
pub fn create_gitlab_release(
  host: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  let (owner, repo) = resolve_owner_repo(cwd)?;
  let encoded_path: String =
    url::form_urlencoded::byte_serialize(format!("{owner}/{repo}").as_bytes()).collect();
  let agent = agent();
  let mut resp = agent
    .get(&format!("{host}/api/v4/projects/{encoded_path}"))
    .header("PRIVATE-TOKEN", token)
    .call()
    .map_err(|e| ReleaseError::Http {
      message: format!("gitlab [open api] error : {e}"),
    })?;
  check_status(&mut resp, Provider::Gitlab)?;
  let project: Value = resp
    .body_mut()
    .read_json()
    .map_err(|e| ReleaseError::Http {
      message: format!("gitlab [open api] error : 项目信息解析失败：{e}"),
    })?;
  let id = project
    .get("id")
    .and_then(Value::as_u64)
    .ok_or_else(|| ReleaseError::Http {
      message: "无法获取项目对应 gitlab 项目 id".into(),
    })?;
  post_json(
    &agent,
    &format!("{host}/api/v4/projects/{id}/releases"),
    &[("PRIVATE-TOKEN", token.to_owned())],
    &json!({
      "name": new_version,
      "tag_name": format!("v{new_version}"),
      "description": markdown,
    }),
    Provider::Gitlab,
  )
}
