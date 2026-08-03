//! Gitee release（ADR-0018 薄文件）：base_url + token 注入形态
//! （请求体 `access_token` 字段）；请求体语义共享自 github_like。

use std::path::Path;

use serde_json::Value;

use super::http::{agent, post_json, resolve_owner_repo};
use super::{github_like, Provider, ReleaseError};

const BASE_URL: &str = "https://gitee.com/api/v5";

/// 生产入口：真实 API 地址
pub(crate) fn create(
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  create_with_base(BASE_URL, token, new_version, markdown, cwd)
}

/// base_url 可注入（测试指向本地 mock）；生产经 `create` 传真实地址
#[doc(hidden)]
pub fn create_with_base(
  base_url: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  let (owner, repo) = resolve_owner_repo(cwd)?;
  let branch = crate::git::get_current_git_branch(cwd)?;
  let mut body = github_like::release_body(new_version, markdown, &branch);
  body["access_token"] = Value::String(token.to_owned());
  let url = github_like::releases_url(base_url, &owner, &repo);
  post_json(&agent(), &url, &[], &body, Provider::Gitee)
}
