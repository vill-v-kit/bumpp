//! GitHub release（ADR-0014 薄文件）：base_url + token 注入形态
//! （`authorization: Bearer` 头 + API 版本头）；请求体语义共享自 github_like。

use std::path::Path;

use super::http::{post_json, resolve_owner_repo};
use super::{github_like, Provider, ReleaseError};
use crate::effects::{Effects, RealEffects};
use crate::git::get_current_git_branch;

const BASE_URL: &str = "https://api.github.com";

/// 生产入口：真实 API 地址
pub(crate) fn create(
  eff: &dyn Effects,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  create_with_base_and(eff, BASE_URL, token, new_version, markdown, cwd)
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
  create_with_base_and(&RealEffects, base_url, token, new_version, markdown, cwd)
}

/// base_url + 效应边界双注入（dry-run 的记录型效应经此骑同一条发送链）
pub(crate) fn create_with_base_and(
  eff: &dyn Effects,
  base_url: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  send(eff, base_url, token, new_version, markdown, cwd).map_err(|e| e.redact(token))
}

fn send(
  eff: &dyn Effects,
  base_url: &str,
  token: &str,
  new_version: &str,
  markdown: &str,
  cwd: &Path,
) -> Result<(), ReleaseError> {
  let (owner, repo) = resolve_owner_repo(cwd)?;
  let branch = get_current_git_branch(cwd)?;
  let body = github_like::release_body(new_version, markdown, &branch);
  let url = github_like::releases_url(base_url, &owner, &repo);
  let headers: [(&str, String); 2] = [
    ("x-github-api-version", "2022-11-28".to_owned()),
    ("authorization", format!("Bearer {token}")),
  ];
  post_json(eff, &url, &headers, &body, Provider::Github)
}
