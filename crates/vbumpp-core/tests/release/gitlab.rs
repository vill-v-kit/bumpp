//! GitLab：`gitlab.host` 配置段（严格 schema：仅 host；overrides 段 > 文件段）
//! + 项目 id 直查 + PRIVATE-TOKEN 的 release 流程

use super::{git_repo, recv, spawn_mock};
use serde_json::{json, Map, Value};
use tempfile::TempDir;

fn doc(v: Value) -> Map<String, Value> {
  v.as_object().unwrap().clone()
}

#[test]
fn gitlab_host_resolution() {
  use vbumpp_core::release::gitlab::resolve_gitlab_host;
  assert_eq!(resolve_gitlab_host(None, None).unwrap(), None);

  let document = doc(json!({ "gitlab": { "host": "https://gitlab.internal" } }));
  assert_eq!(
    resolve_gitlab_host(Some(&document), None)
      .unwrap()
      .as_deref(),
    Some("https://gitlab.internal")
  );

  let overrides = doc(json!({ "gitlab": { "host": "https://override.example" } }));
  assert_eq!(
    resolve_gitlab_host(Some(&document), Some(&overrides))
      .unwrap()
      .as_deref(),
    Some("https://override.example"),
    "overrides 段优先"
  );
}

#[test]
fn gitlab_section_strict_schema() {
  use vbumpp_core::release::gitlab::resolve_gitlab_host;
  let unknown = doc(json!({ "gitlab": { "token": "x" } }));
  let err = resolve_gitlab_host(Some(&unknown), None).unwrap_err();
  assert!(
    err.to_string().contains("unsupported key \"token\""),
    "{err}"
  );

  let not_object = doc(json!({ "gitlab": "https://x" }));
  assert!(resolve_gitlab_host(Some(&not_object), None).is_err());

  let bad_type = doc(json!({ "gitlab": { "host": 42 } }));
  assert!(resolve_gitlab_host(Some(&bad_type), None).is_err());
}

#[test]
fn gitlab_looks_up_project_id_then_posts_release() {
  let mock = spawn_mock(|req| {
    if req.target.starts_with("/api/v4/projects/owner") {
      (200, r#"{"id":42}"#.to_owned())
    } else {
      (201, "{}".to_owned())
    }
  });
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitlab.com:owner/repo.git");
  vbumpp_core::release::gitlab::create_with_host(
    &format!("http://{}", mock.addr),
    "gl-token",
    "3.1.4",
    "notes md",
    &cwd,
  )
  .unwrap();

  let lookup = recv(&mock);
  assert_eq!(lookup.method, "GET");
  assert_eq!(
    lookup.target, "/api/v4/projects/owner%2Frepo",
    "owner/repo url 编码直查（替代 JS 时代的搜索 + 后缀匹配）"
  );
  assert_eq!(lookup.header("PRIVATE-TOKEN"), Some("gl-token"));

  let post = recv(&mock);
  assert_eq!(post.method, "POST");
  assert_eq!(post.target, "/api/v4/projects/42/releases");
  let body = post.json();
  assert_eq!(body["tag_name"], "v3.1.4");
  assert_eq!(body["name"], "3.1.4");
  assert_eq!(body["description"], "notes md");
}

#[test]
fn gitlab_project_lookup_404_errors() {
  let mock = spawn_mock(|_| (404, r#"{"message":"404 Project Not Found"}"#.to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitlab.com:owner/repo.git");
  let err = vbumpp_core::release::gitlab::create_with_host(
    &format!("http://{}", mock.addr),
    "glpat-secret-token",
    "1.0.0",
    "",
    &cwd,
  )
  .unwrap_err();
  assert!(err.to_string().contains("404 Project Not Found"), "{err}");
}
