//! GitHub：Bearer 头注入 + API 版本头；共享请求体语义（tag_name v 前缀、
//! target_commitish、prerelease 判定）经 github 注入缝锚定

use super::{git_repo, recv, spawn_mock};
use tempfile::TempDir;

#[test]
fn github_posts_release_with_bearer_headers() {
  let mock = spawn_mock(|_| (201, r#"{"id":1}"#.to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@github.com:owner/repo.git");
  bumpp_core::release::github::create_with_base(
    &format!("http://{}", mock.addr),
    "secret-token",
    "1.2.3",
    "## v1.2.3\n\n- feat: x",
    &cwd,
  )
  .unwrap();

  let req = recv(&mock);
  assert_eq!(req.method, "POST");
  assert_eq!(req.target, "/repos/owner/repo/releases");
  assert_eq!(req.header("authorization"), Some("Bearer secret-token"));
  assert_eq!(req.header("x-github-api-version"), Some("2022-11-28"));
  let body = req.json();
  assert_eq!(body["tag_name"], "v1.2.3");
  assert_eq!(body["name"], "1.2.3");
  assert_eq!(body["body"], "## v1.2.3\n\n- feat: x");
  assert_eq!(body["target_commitish"], "main");
  assert_eq!(body["prerelease"], false, "正式版非 prerelease");
}

#[test]
fn prerelease_flag_follows_version_shape() {
  let mock = spawn_mock(|_| (201, "{}".to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@github.com:owner/repo.git");
  bumpp_core::release::github::create_with_base(
    &format!("http://{}", mock.addr),
    "t",
    "1.2.3-beta.1",
    "",
    &cwd,
  )
  .unwrap();
  assert_eq!(
    recv(&mock).json()["prerelease"],
    true,
    "beta/alpha 判 prerelease"
  );
}
