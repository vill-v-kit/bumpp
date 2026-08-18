//! GitLab release：全特化——`PRIVATE-TOKEN` 头 + 项目 id 直查
//! （`GET /api/v4/projects/<url编码的 owner/repo>`，替代 JS 时代的搜索 + 后缀
//! 匹配两步法）；自建实例经配置 `gitlab.host` 段（严格 schema：仅 host 一键，
//! ），解析随本文件。

use std::path::Path;

use serde_json::{json, Map, Value};

use super::http::{check_status, get, post_json, resolve_owner_repo};
use super::{Provider, ReleaseError};
use crate::config::{custom_config_path, gitlab_host_of, read_document};
use crate::effects::{Effects, RealEffects};

/// 缺省 host（有效 host 的唯一字面量维护点——token 键化与 API 拼接不得漂移）
const DEFAULT_HOST: &str = "https://gitlab.com";

/// 生产入口：读配置文档 → 解析 `gitlab.host`（缺省 gitlab.com）→ 创建
pub(crate) fn create(
  eff: &dyn Effects,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
  overrides: Option<&Map<String, Value>>,
) -> Result<(), ReleaseError> {
  let document = read_document(cwd, custom_config_path(overrides).as_deref())?;
  let host = effective_host(document.as_ref(), overrides)?;
  create_with_host_and(eff, &host, token, new_version, markdown, cwd)
}

/// 有效 host：四层合并配置的 `gitlab.host`，缺省 gitlab.com。token 键化
/// （release 解析链提前解析）与 API 拼接（`create`）同一事实源——两侧各自
/// 读档但都归一到本函数，键化与调用永不漂移
pub(crate) fn effective_host(
  document: Option<&Map<String, Value>>,
  overrides: Option<&Map<String, Value>>,
) -> Result<String, ReleaseError> {
  Ok(resolve_gitlab_host(document, overrides)?.unwrap_or_else(|| DEFAULT_HOST.to_owned()))
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
    if let Some(h) = gitlab_host_of(source)? {
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
  create_with_host_and(&RealEffects, host, token, new_version, markdown, cwd)
}

/// host + 效应边界双注入（dry-run 的记录型效应经此骑同一条发送链）
pub(crate) fn create_with_host_and(
  eff: &dyn Effects,
  host: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  send(eff, host, token, new_version, markdown, cwd).map_err(|e| e.redact(token))
}

fn send(
  eff: &dyn Effects,
  host: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  let (owner, repo) = resolve_owner_repo(cwd)?;
  let encoded_path: String =
    url::form_urlencoded::byte_serialize(format!("{owner}/{repo}").as_bytes()).collect();
  let resp = get(
    eff,
    &format!("{host}/api/v4/projects/{encoded_path}"),
    &[("PRIVATE-TOKEN", token.to_owned())],
    Provider::Gitlab,
  )?;
  check_status(&resp, Provider::Gitlab)?;
  // 解析失败的文案保持 ureq `read_json` 时代的 Display 形态（`json: {serde}`）
  let project: Value = serde_json::from_str(&resp.body).map_err(|e| ReleaseError::Http {
    message: format!("gitlab [open api] error : failed to parse project info: json: {e}"),
  })?;
  let id = project
    .get("id")
    .and_then(Value::as_u64)
    .ok_or_else(|| ReleaseError::Http {
      message: "cannot resolve the gitlab project id".into(),
    })?;
  post_json(
    eff,
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
