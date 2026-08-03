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
