//! 四家 provider 的共用原语：仓库信息解析与 HTTP 收发收尾。
//! 请求构造的语义归 github_like.rs / 各 provider 文件；传输本体在效应边界
//! （`effects.rs` 的 RealEffects），本文件只管「往哪发」的仓库推断、
//! 「怎么发」的传输归口与「非 2xx 怎么报错」。

use std::path::Path;

use serde_json::Value;

use super::{Provider, ReleaseError};
use crate::effects::{Effects, HttpResponse};
use crate::git::resolve_repo_config;

/// `owner/repo` 解析（package.json `repository` 优先、git remote 兜底）
pub(crate) fn resolve_owner_repo(cwd: &Path) -> Result<(String, String), ReleaseError> {
  let repo = resolve_repo_config(cwd)
    .and_then(|r| r.repo)
    .ok_or_else(|| ReleaseError::Git {
      message: "cannot resolve the remote repository".into(),
    })?;
  repo
    .split_once('/')
    .map(|(o, r)| (o.to_owned(), r.to_owned()))
    .ok_or_else(|| ReleaseError::Git {
      message: "cannot resolve the remote repository".into(),
    })
}

/// GET（经效应边界）：传输失败按各家共同的 `{provider} [open api] error : {e}` 归口；
/// 状态码裁决与响应体消费归调用方
pub(crate) fn get(
  eff: &dyn Effects,
  url: &str,
  headers: &[(&str, String)],
  provider: Provider,
) -> Result<HttpResponse, ReleaseError> {
  eff.http_get(url, headers).map_err(|e| ReleaseError::Http {
    message: format!("{} [open api] error : {e}", provider.display()),
  })
}

/// POST JSON + 非 2xx 检查（经效应边界）：四家共用的收发收尾
pub(crate) fn post_json(
  eff: &dyn Effects,
  url: &str,
  headers: &[(&str, String)],
  body: &Value,
  provider: Provider,
) -> Result<(), ReleaseError> {
  let resp = eff
    .http_post_json(url, headers, body)
    .map_err(|e| ReleaseError::Http {
      message: format!("{} [open api] error : {e}", provider.display()),
    })?;
  check_status(&resp, provider)
}

/// 非 2xx 报错：提取服务端 `message` 字段（gitlab/gitee 错误体形态），
/// 无法解析时回落原始响应体
pub(crate) fn check_status(resp: &HttpResponse, provider: Provider) -> Result<(), ReleaseError> {
  if (200..300).contains(&resp.status) {
    return Ok(());
  }
  let server_message = serde_json::from_str::<Value>(&resp.body)
    .ok()
    .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_owned))
    .filter(|m| !m.is_empty())
    .unwrap_or_else(|| {
      if resp.body.is_empty() {
        "Unknown error".into()
      } else {
        resp.body.clone()
      }
    });
  Err(ReleaseError::Http {
    message: format!(
      "{} [open api] error : [{}] {server_message}",
      provider.display(),
      resp.status
    ),
  })
}
