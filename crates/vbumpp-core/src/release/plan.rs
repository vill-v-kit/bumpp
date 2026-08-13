//! release 计划预览（COL-84 dry-run 的核心）：以记录型效应（PreviewEffects）
//! 骑真实创建链（`dispatch`，预演与执行同路）——请求构造、gitlab 的 GET
//! project id 两跳、错误归口全部与真实执行一致，HTTP 在边界被拦截为计划
//! 条目（零网络收发）。装配产物 `ReleasePlan` 只携带展示所需字段：
//! 明文 token 不出本函数（拦截条目中的注入形态按 `ReleaseError::redact`
//! 同规则脱敏为 `[redacted]`，原始与 form 编码两形态都替换）。

use std::io;
use std::path::Path;
use std::sync::Mutex;

use serde_json::{Map, Value};

use super::{github_like, http, Provider, ReleaseError, TokenSource};
use crate::effects::{Effects, HttpResponse};
use crate::exec::ExecError;

/// 计划中的一条平台请求（被边界拦截的 HTTP 调用）：方法 + 脱敏后的 URL
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRequest {
  pub method: &'static str,
  pub url: String,
}

/// release dry-run 的装配产物：token 来源（无明文）+ 目标信息 + 请求计划
#[derive(Debug, Clone)]
pub struct ReleasePlan {
  pub provider: Provider,
  /// token 解析链命中的来源；None = 缺失（CLI 降级为警告行，预览照常）
  pub token_source: Option<TokenSource>,
  /// 平台 API 基址（gitlab 为配置的 `gitlab.host`，含自建实例路径）
  pub host: String,
  pub owner: String,
  pub repo: String,
  pub tag_name: String,
  /// prerelease 判定（beta/alpha，与 github-like 请求体同一事实源）
  pub prerelease: bool,
  /// body 全文 = 提取的 changelog 版本节
  pub body: String,
  /// 边界拦截到的请求序列（gitlab 为 GET project id → POST releases 两跳）
  pub requests: Vec<PlannedRequest>,
}

/// 记录型效应（dry-run 注入）：HTTP 只记录不收发；GET 应答合成 project id
/// 供 gitlab 链续走（POST URL 消费该 id——展示的 id 为占位值，真实 id 只有
/// 真实执行才能查得）。release 链不触发文件写盘与子进程，两原语空实现
struct PreviewEffects {
  requests: Mutex<Vec<PlannedRequest>>,
}

impl PreviewEffects {
  fn new() -> Self {
    Self {
      requests: Mutex::new(Vec::new()),
    }
  }

  fn record(&self, method: &'static str, url: &str) {
    self.requests.lock().unwrap().push(PlannedRequest {
      method,
      url: url.to_owned(),
    });
  }

  fn into_requests(self) -> Vec<PlannedRequest> {
    self.requests.into_inner().unwrap()
  }
}

impl Effects for PreviewEffects {
  fn write_file(&self, _path: &Path, _content: &str) -> io::Result<()> {
    Ok(())
  }

  fn run(&self, _program: &str, _args: &[String], _cwd: &Path) -> Result<(), ExecError> {
    Ok(())
  }

  fn http_get(&self, url: &str, _headers: &[(&str, String)]) -> Result<HttpResponse, String> {
    self.record("GET", url);
    Ok(HttpResponse {
      status: 200,
      body: r#"{"id":0}"#.to_owned(),
    })
  }

  fn http_post_json(
    &self,
    url: &str,
    _headers: &[(&str, String)],
    _body: &Value,
  ) -> Result<HttpResponse, String> {
    self.record("POST", url);
    Ok(HttpResponse {
      status: 201,
      body: "{}".to_owned(),
    })
  }
}

/// 平台 API 基址：从拦截 URL 反推。与各家 URL 形状的耦合钉在此处——
/// github / gitee / gitcode：`{base_url}/repos/{owner}/{repo}/releases`
/// （github_like::releases_url），剥 `/repos/` 后缀即 base_url；
/// gitlab：`{host}/api/v4/projects/...`（gitlab.rs），剥 `/api/v4/` 后缀即
/// 配置的 host（自建实例的路径前缀随之保留）。provider URL 形状变更时
/// 必须同步本函数
fn host_of(provider: Provider, requests: &[PlannedRequest]) -> String {
  const EMPTY: &str = "";
  let strip = |url: &str, marker: &str| {
    url
      .find(marker)
      .map(|index| url[..index].to_owned())
      .unwrap_or_else(|| url.to_owned())
  };
  match provider {
    Provider::Gitlab => requests
      .first()
      .map(|r| strip(&r.url, "/api/v4/"))
      .unwrap_or_else(|| EMPTY.to_owned()),
    _ => requests
      .iter()
      .find(|r| r.method == "POST")
      .map(|r| strip(&r.url, "/repos/"))
      .unwrap_or_else(|| EMPTY.to_owned()),
  }
}

/// release dry-run 的计划装配：token 宽容解析（缺失不报错）→ 记录型效应骑
/// 真实创建链（校验与请求构造全走，HTTP 零收发）→ 收集展示字段
pub fn plan_release(
  provider: Provider,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
  overrides: Option<&Map<String, Value>>,
) -> Result<ReleasePlan, ReleaseError> {
  let resolved = super::resolve_token_tolerant(provider, cwd);
  let preview = PreviewEffects::new();
  // token 缺失以空串占位续走同一条链——占位值只落在不展示的头部/请求体字段
  let empty = String::new();
  let token = resolved
    .as_ref()
    .map(|r| r.token.as_str())
    .unwrap_or(&empty);
  super::dispatch(
    &preview,
    provider,
    token,
    new_version,
    markdown,
    cwd,
    overrides,
  )?;

  // owner/repo 与链内同源（只读推断；dispatch 已成功即此步必成，仍按可错处理）
  let (owner, repo) = http::resolve_owner_repo(cwd)?;
  // 拦截条目脱敏（gitcode 经 query 注入 token）——与报错同一脱敏原语
  let requests: Vec<PlannedRequest> = preview
    .into_requests()
    .into_iter()
    .map(|r| PlannedRequest {
      method: r.method,
      url: super::scrub_token(&r.url, token),
    })
    .collect();
  let host = host_of(provider, &requests);
  Ok(ReleasePlan {
    provider,
    token_source: resolved.map(|r| r.source),
    host,
    owner,
    repo,
    tag_name: format!("v{new_version}"),
    prerelease: github_like::is_prerelease(new_version),
    body: markdown.to_owned(),
    requests,
  })
}
