//! Gitee：token 注入请求体 `access_token` 字段（不走 Bearer 头）

use super::{git_repo, recv, spawn_mock};
use tempfile::TempDir;

#[test]
fn gitee_injects_token_into_body() {
  let mock = spawn_mock(|_| (201, "{}".to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitee.com:owner/repo.git");
  bumpp_core::release::gitee::create_with_base(
    &format!("http://{}", mock.addr),
    "gitee-token",
    "2.0.0",
    "notes",
    &cwd,
  )
  .unwrap();
  let req = recv(&mock);
  assert_eq!(req.header("authorization"), None, "gitee 不走 Bearer 头");
  assert_eq!(req.json()["access_token"], "gitee-token");
}
