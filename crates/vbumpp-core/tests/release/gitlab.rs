//! GitLab：`gitlab.host` 配置段（严格 schema：仅 host；overrides 段 > 文件段）、
//! 项目 id 直查、PRIVATE-TOKEN 的 release 流程；host 作用域 token 键的四级
//! 解析链（精确键 → provider 级回落 → GITLAB_TOKEN → 报错）走真实入口验证

use super::{git_repo, recv, sanitized_token_env, spawn_mock};
use serde_json::{json, Map, Value};
use tempfile::TempDir;
use vbumpp_core::release::{create_release, Provider};
use vbumpp_core::token::{host_scoped_key, save_token_at};

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

// ---------------------------------------------------------------------------
// host 作用域 token 键四级解析链：真实入口（create_release）+ mock server
// 断言 PRIVATE-TOKEN 头值
// ---------------------------------------------------------------------------

/// 配置 `gitlab.host` 指向 mock 的仓库（真实入口经 .vbumpprc.json 解析有效 host）
fn gitlab_repo_with_host(dir: &TempDir, host: &str) -> std::path::PathBuf {
  let path = git_repo(dir, "git@gitlab.com:owner/repo.git");
  std::fs::write(
    path.join(".vbumpprc.json"),
    format!("{{\n  \"gitlab\": {{\n    \"host\": \"{host}\"\n  }}\n}}\n"),
  )
  .unwrap();
  path
}

/// 项目 id 直查 + 创建两跳都放行（目标匹配容忍 host 尾斜杠带来的双斜杠）
fn spawn_gitlab_mock() -> super::Mock {
  spawn_mock(|req| {
    if req.method == "GET" && req.target.contains("/api/v4/projects/") {
      (200, r#"{"id":42}"#.to_owned())
    } else {
      (201, "{}".to_owned())
    }
  })
}

#[test]
fn host_scoped_store_key_wins_via_real_entry() {
  let mock = spawn_gitlab_mock();
  let dir = TempDir::new().unwrap();
  let host = format!("http://{}", mock.addr);
  let cwd = gitlab_repo_with_host(&dir, &host);
  let store = dir.path().join("tokens.bin");
  let _guard = sanitized_token_env(&store);
  save_token_at(
    &store,
    &host_scoped_key("gitlab", &host).unwrap(),
    "scoped-token",
  )
  .unwrap();
  save_token_at(&store, "gitlab", "provider-token").unwrap();

  create_release(Provider::Gitlab, "3.1.4", "notes md", &cwd, None).unwrap();

  let lookup = recv(&mock);
  assert_eq!(
    lookup.header("PRIVATE-TOKEN"),
    Some("scoped-token"),
    "host 作用域精确键优先于 provider 级键"
  );
}

#[test]
fn provider_key_fallback_via_real_entry() {
  // 向后兼容硬要求：存量自建 GitLab 用户的 token 都在 provider 级键下
  let mock = spawn_gitlab_mock();
  let dir = TempDir::new().unwrap();
  let host = format!("http://{}", mock.addr);
  let cwd = gitlab_repo_with_host(&dir, &host);
  let store = dir.path().join("tokens.bin");
  let _guard = sanitized_token_env(&store);
  save_token_at(&store, "gitlab", "provider-token").unwrap();

  create_release(Provider::Gitlab, "3.1.4", "notes md", &cwd, None).unwrap();

  let lookup = recv(&mock);
  assert_eq!(lookup.header("PRIVATE-TOKEN"), Some("provider-token"));
}

#[test]
fn env_fallback_via_real_entry() {
  let mock = spawn_gitlab_mock();
  let dir = TempDir::new().unwrap();
  let host = format!("http://{}", mock.addr);
  let cwd = gitlab_repo_with_host(&dir, &host);
  let store = dir.path().join("tokens.bin");
  let _guard = sanitized_token_env(&store);
  std::env::set_var("GITLAB_TOKEN", "env-token-gl");

  create_release(Provider::Gitlab, "3.1.4", "notes md", &cwd, None).unwrap();
  std::env::remove_var("GITLAB_TOKEN");

  let lookup = recv(&mock);
  assert_eq!(lookup.header("PRIVATE-TOKEN"), Some("env-token-gl"));
}

#[test]
fn trailing_slash_config_value_collides_via_real_entry() {
  // 写入走真实 CLI `token set --host` 通路（无尾斜杠），读取侧配置带尾
  // 斜杠：两侧各自经同一规范化函数归一相撞——不是同函数双调的构造性相等，
  // 而是写读两条真实通路的相撞。API 调用仍用原始 host 串拼接（双斜杠
  // 路径，调用行为不变）
  let mock = spawn_gitlab_mock();
  let dir = TempDir::new().unwrap();
  let host = format!("http://{}", mock.addr);
  let cwd = gitlab_repo_with_host(&dir, &format!("{host}/"));
  let store = dir.path().join("tokens.bin");
  let _guard = sanitized_token_env(&store);
  let argv: Vec<String> = ["token", "set", "gitlab", "--host", &host]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
  let prompt = |_name: &str| Ok(Some("scoped-token".to_string()));
  let env = vbumpp_core::cli::RunEnv {
    store: Some(&store),
    cwd: None,
    home: None,
    prompt: Some(&prompt),
    confirm: None,
  };
  let mut out = Vec::new();
  let mut err_buf = Vec::new();
  let code = vbumpp_core::cli::run_at(&argv, None, &env, &mut out, &mut err_buf);
  assert_eq!(
    code,
    0,
    "token set 写入：{}",
    String::from_utf8_lossy(&err_buf)
  );

  create_release(Provider::Gitlab, "3.1.4", "notes md", &cwd, None).unwrap();

  let lookup = recv(&mock);
  assert_eq!(lookup.header("PRIVATE-TOKEN"), Some("scoped-token"));
}

#[test]
fn missing_token_error_carries_host_guidance() {
  let mock = spawn_gitlab_mock();
  let dir = TempDir::new().unwrap();
  let host = format!("http://{}", mock.addr);
  let cwd = gitlab_repo_with_host(&dir, &host);
  let store = dir.path().join("tokens.bin");
  let _guard = sanitized_token_env(&store);

  let err = create_release(Provider::Gitlab, "3.1.4", "notes md", &cwd, None).unwrap_err();
  assert_eq!(
    err.to_string(),
    format!(
      "no Gitlab token detected for {host}; run vbumpp token set gitlab --host {host} to add one"
    ),
    "报错带 host 指引"
  );
}

#[test]
fn scoped_token_never_leaks_into_error_messages() {
  // redact 语义保持：服务端回显 token 的错误经同一脱敏原语处理
  let mock = spawn_mock(|_| {
    (
      400,
      r#"{"message":"bad request for token gl-scoped-xyz"}"#.to_owned(),
    )
  });
  let dir = TempDir::new().unwrap();
  let host = format!("http://{}", mock.addr);
  let cwd = gitlab_repo_with_host(&dir, &host);
  let store = dir.path().join("tokens.bin");
  let _guard = sanitized_token_env(&store);
  save_token_at(
    &store,
    &host_scoped_key("gitlab", &host).unwrap(),
    "gl-scoped-xyz",
  )
  .unwrap();

  let err = create_release(Provider::Gitlab, "3.1.4", "notes md", &cwd, None).unwrap_err();
  let msg = err.to_string();
  assert!(
    !msg.contains("gl-scoped-xyz"),
    "明文 token 不出错误消息：{msg}"
  );
  assert!(msg.contains("[redacted]"), "{msg}");
}
