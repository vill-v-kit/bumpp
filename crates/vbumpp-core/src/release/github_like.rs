//! github-like 共享机器（ADR-0018）：github / gitee / gitcode 的 release 请求体
//! 语义单一事实源——三家 API 同形（`POST /repos/{owner}/{repo}/releases`，
//! name/tag_name/body/target_commitish/prerelease），差异仅 token 注入形态，
//! 归各 provider 薄文件。

use std::sync::LazyLock;

use serde_json::{json, Value};

static PRERELEASE_RE: LazyLock<regex::Regex> =
  LazyLock::new(|| regex::Regex::new(r"(beta|alpha)").unwrap());

/// releases 端点 URL（token 的 query 注入由 provider 文件在此返回值上追加）
pub(crate) fn releases_url(base_url: &str, owner: &str, repo: &str) -> String {
  format!("{base_url}/repos/{owner}/{repo}/releases")
}

/// 共享请求体：beta/alpha 版本号判 prerelease
pub(crate) fn release_body(new_version: &str, markdown: &str, branch: &str) -> Value {
  json!({
    "name": new_version,
    "tag_name": format!("v{new_version}"),
    "body": markdown,
    "target_commitish": branch,
    "prerelease": PRERELEASE_RE.is_match(new_version),
  })
}
