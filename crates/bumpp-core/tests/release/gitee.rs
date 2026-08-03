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

#[test]
fn gitee_error_never_leaks_token() {
  // 服务端错误回显请求体——gitee 的 token 在 body access_token 字段
  // （原始形态），报错必须脱敏（ADR-0014）
  let mock = spawn_mock(|req| {
    (
      400,
      serde_json::json!({ "message": format!("invalid payload: {}", req.body) }).to_string(),
    )
  });
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitee.com:owner/repo.git");
  let err = bumpp_core::release::gitee::create_with_base(
    &format!("http://{}", mock.addr),
    "gitee-secret-token",
    "2.0.0",
    "notes",
    &cwd,
  )
  .unwrap_err();
  let msg = err.to_string();
  assert!(!msg.contains("gitee-secret-token"), "原始形态泄漏：{msg}");
  assert!(msg.contains("[redacted]"), "掩码应在场：{msg}");
}
