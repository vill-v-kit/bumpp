//! 四家 provider 的共用原语（ADR-0018）：仓库信息解析与 HTTP 收发。
//! 请求构造的语义归 github_like.rs / 各 provider 文件，本文件只管
//! 「往哪发、怎么发、非 2xx 怎么报错」。

use std::path::Path;

use serde_json::Value;

use super::{Provider, ReleaseError};

/// `owner/repo` 解析（package.json `repository` 优先、git remote 兜底）
pub(crate) fn resolve_owner_repo(cwd: &Path) -> Result<(String, String), ReleaseError> {
  let repo = crate::git::resolve_repo_config(cwd)
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

/// 共享 Agent：30s 全局超时；状态码不当异常（手动检查以提取服务端错误信息）
pub(crate) fn agent() -> ureq::Agent {
  ureq::Agent::config_builder()
    .timeout_global(Some(std::time::Duration::from_secs(30)))
    .http_status_as_error(false)
    .build()
    .into()
}

/// 非 2xx 报错：提取服务端 `message` 字段（gitlab/gitee 错误体形态），
/// 无法解析时回落原始响应体
pub(crate) fn check_status(
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

pub(crate) fn post_json(
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
