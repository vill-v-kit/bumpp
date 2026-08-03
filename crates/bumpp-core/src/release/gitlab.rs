//! GitLab release（ADR-0018）：全特化——`PRIVATE-TOKEN` 头 + 项目 id 直查
//! （`GET /api/v4/projects/<url编码的 owner/repo>`，替代 JS 时代的搜索 + 后缀
//! 匹配两步法）；自建实例经配置 `gitlab.host` 段（严格 schema：仅 host 一键，
//! ADR-0014），解析随本文件。

use std::path::Path;

use serde_json::{json, Map, Value};

use super::http::{agent, check_status, post_json, resolve_owner_repo};
use super::{Provider, ReleaseError};

/// 生产入口：读配置文档 → 解析 `gitlab.host`（缺省 gitlab.com）→ 创建
pub(crate) fn create(
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
  overrides: Option<&Map<String, Value>>,
) -> Result<(), ReleaseError> {
  let document =
    crate::config::read_document(cwd, crate::config::custom_config_path(overrides).as_deref())?;
  let host =
    resolve_gitlab_host(document.as_ref(), overrides)?.unwrap_or_else(|| "https://gitlab.com".to_owned());
  create_with_host(&host, token, new_version, markdown, cwd)
}

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

/// host 可注入（测试指向本地 mock）；生产经 `create` 解析 `gitlab.host`
#[doc(hidden)]
pub fn create_with_host(
  host: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  send(host, token, new_version, markdown, cwd).map_err(|e| e.redact(token))
}

fn send(
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
      message: format!("gitlab [open api] error : failed to parse project info: {e}"),
    })?;
  let id = project
    .get("id")
    .and_then(Value::as_u64)
    .ok_or_else(|| ReleaseError::Http {
      message: "cannot resolve the gitlab project id".into(),
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
