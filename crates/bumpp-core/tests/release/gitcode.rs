//! GitCode：token 注入 query `access_token`（form 编码对齐 JS URLSearchParams）

use super::{git_repo, recv, spawn_mock};
use tempfile::TempDir;

#[test]
fn gitcode_injects_token_into_query() {
  let mock = spawn_mock(|_| (201, "{}".to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitcode.com:owner/repo.git");
  bumpp_core::release::gitcode::create_with_base(
    &format!("http://{}", mock.addr),
    "gitcode token+",
    "2.0.0",
    "notes",
    &cwd,
  )
  .unwrap();
  let req = recv(&mock);
  assert!(
    req.target.contains("access_token=gitcode+token%2B"),
    "query 携带 form 编码 token（对齐 JS URLSearchParams 的空格→+）：{}",
    req.target
  );
}

#[test]
fn gitcode_error_never_leaks_token() {
  // 服务端错误回显请求目标——gitcode 的 token 在 URL query 里（form 编码形态），
  // 报错必须脱敏（ADR-0014：明文 token 不出模块，对错误消息同样成立）
  let mock = spawn_mock(|req| {
    (
      422,
      serde_json::json!({ "message": format!("bad request target {}", req.target) }).to_string(),
    )
  });
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitcode.com:owner/repo.git");
  let err = bumpp_core::release::gitcode::create_with_base(
    &format!("http://{}", mock.addr),
    "gitcode token+",
    "2.0.0",
    "notes",
    &cwd,
  )
  .unwrap_err();
  let msg = err.to_string();
  assert!(!msg.contains("gitcode token+"), "原始形态泄漏：{msg}");
  assert!(!msg.contains("gitcode+token%2B"), "form 编码形态泄漏：{msg}");
  assert!(msg.contains("[redacted]"), "掩码应在场：{msg}");
}
