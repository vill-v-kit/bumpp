//! 平台 Release（ADR-0014）：token 解析链 / gitlab.host schema 纯测 +
//! 四家 API 的 mock server 行为测试（零外部依赖的手写 HTTP mock）。

mod common;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc::{channel, Receiver};

use bumpp_core::release::{
  create_github_like_release, create_gitlab_release, resolve_gitlab_host, resolve_token, Provider,
};
use serde_json::{json, Map, Value};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// 手写 HTTP mock：每连接一个请求，记录后经 mpsc 回传，响应由闭包按请求决定
// ---------------------------------------------------------------------------

struct Recorded {
  method: String,
  target: String,
  headers: Vec<(String, String)>,
  body: String,
}

impl Recorded {
  fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(n, _)| n.eq_ignore_ascii_case(name))
      .map(|(_, v)| v.as_str())
  }

  fn json(&self) -> Value {
    serde_json::from_str(&self.body).unwrap()
  }
}

struct Mock {
  addr: SocketAddr,
  received: Receiver<Recorded>,
}

fn spawn_mock(respond: impl Fn(&Recorded) -> (u16, String) + Send + 'static) -> Mock {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  let (tx, rx) = channel();
  std::thread::spawn(move || {
    for stream in listener.incoming() {
      let mut stream = stream.unwrap();
      stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
      let mut buf = Vec::new();
      let mut chunk = [0u8; 4096];
      // 读到头部结束
      let header_end = loop {
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
          break pos;
        }
        let n = stream.read(&mut chunk).unwrap();
        if n == 0 {
          panic!("连接在头部完成前关闭");
        }
        buf.extend_from_slice(&chunk[..n]);
      };
      let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
      let mut lines = head.lines();
      let mut request_line = lines.next().unwrap().split_whitespace();
      let method = request_line.next().unwrap().to_owned();
      let target = request_line.next().unwrap().to_owned();
      let headers: Vec<(String, String)> = lines
        .filter_map(|line| {
          line
            .split_once(':')
            .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        })
        .collect();
      let content_length: usize = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
      let body_start = header_end + 4;
      while buf.len() < body_start + content_length {
        let n = stream.read(&mut chunk).unwrap();
        if n == 0 {
          break;
        }
        buf.extend_from_slice(&chunk[..n]);
      }
      let recorded = Recorded {
        method,
        target,
        headers,
        body: String::from_utf8_lossy(
          &buf[body_start..body_start + content_length.min(buf.len() - body_start)],
        )
        .into_owned(),
      };
      let (status, response_body) = respond(&recorded);
      tx.send(recorded).unwrap();
      let reason = if status < 300 { "OK" } else { "Error" };
      write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
        response_body.len()
      )
      .unwrap();
    }
  });
  Mock { addr, received: rx }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
  haystack.windows(needle.len()).position(|w| w == needle)
}

fn recv(mock: &Mock) -> Recorded {
  mock
    .received
    .recv_timeout(std::time::Duration::from_secs(5))
    .unwrap()
}

/// 带一次提交的 git 仓库 + origin remote（resolve_owner_repo / get_current_git_branch 的前提）
fn git_repo(dir: &TempDir, remote: &str) -> std::path::PathBuf {
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  std::fs::write(path.join("f.txt"), "x\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  common::git(&path, &["remote", "add", "origin", remote]);
  path
}

// ---------------------------------------------------------------------------
// token 解析链（ADR-0014）：store → 环境变量 →（仅 github）gh CLI
// ---------------------------------------------------------------------------

fn tokens(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
  pairs
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

fn no_env(_: &str) -> Option<String> {
  None
}

fn no_gh() -> Option<String> {
  None
}

#[test]
fn store_token_wins_over_env() {
  let env = |_: &str| Some("env-token".to_owned());
  let got = resolve_token(
    Provider::Github,
    &tokens(&[("github", "store-token")]),
    &env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("store-token"));
}

#[test]
fn empty_store_value_falls_through_to_env() {
  // JS `||` 语义：空字符串视为缺失
  let env = |key: &str| (key == "GH_TOKEN").then(|| "env-token".to_owned());
  let got = resolve_token(Provider::Github, &tokens(&[("github", "")]), &env, &no_gh);
  assert_eq!(got.as_deref(), Some("env-token"));
}

#[test]
fn github_env_chain_order_and_fixed_spelling() {
  // GH_TOKEN 优先；GITHUB_TOKEN 次之（GITHOB_TOKEN 已移除——不再被读取）
  let both = |key: &str| match key {
    "GH_TOKEN" => Some("gh-token".to_owned()),
    "GITHUB_TOKEN" => Some("github-token".to_owned()),
    "GITHOB_TOKEN" => Some("typo-token".to_owned()),
    _ => None,
  };
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &both, &no_gh);
  assert_eq!(got.as_deref(), Some("gh-token"));

  let only_github = |key: &str| match key {
    "GITHUB_TOKEN" => Some("github-token".to_owned()),
    "GITHOB_TOKEN" => Some("typo-token".to_owned()),
    _ => None,
  };
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &only_github, &no_gh);
  assert_eq!(got.as_deref(), Some("github-token"));

  let only_typo = |key: &str| (key == "GITHOB_TOKEN").then(|| "typo-token".to_owned());
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &only_typo, &no_gh);
  assert_eq!(got, None, "拼错的 GITHOB_TOKEN 不再生效");
}

#[test]
fn github_gh_cli_fallback_is_trimmed() {
  let gh = || Some("cli-token\n".to_owned());
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &no_env, &gh);
  assert_eq!(got.as_deref(), Some("cli-token"));
}

#[test]
fn other_providers_have_own_env_and_no_gh_cli() {
  let env = |key: &str| match key {
    "GITLAB_TOKEN" => Some("gl".to_owned()),
    "GITEE_TOKEN" => Some("ge".to_owned()),
    "GITCODE_TOKEN" => Some("gc".to_owned()),
    _ => None,
  };
  let gh_called = || panic!("非 github 不应触达 gh CLI");
  assert_eq!(
    resolve_token(Provider::Gitlab, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("gl")
  );
  assert_eq!(
    resolve_token(Provider::Gitee, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("ge")
  );
  assert_eq!(
    resolve_token(Provider::Gitcode, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("gc")
  );
}

#[test]
fn store_key_is_per_provider() {
  let got = resolve_token(
    Provider::Gitlab,
    &tokens(&[("gitlab", "stored")]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("stored"));
  assert_eq!(
    resolve_token(
      Provider::Gitee,
      &tokens(&[("gitlab", "stored")]),
      &no_env,
      &no_gh
    ),
    None
  );
}

// ---------------------------------------------------------------------------
// gitlab.host 配置段（严格 schema：仅 host；overrides 段 > 文件段）
// ---------------------------------------------------------------------------

fn doc(v: Value) -> Map<String, Value> {
  v.as_object().unwrap().clone()
}

#[test]
fn gitlab_host_resolution() {
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
  let unknown = doc(json!({ "gitlab": { "token": "x" } }));
  let err = resolve_gitlab_host(Some(&unknown), None).unwrap_err();
  assert!(err.to_string().contains("未支持的键 \"token\""), "{err}");

  let not_object = doc(json!({ "gitlab": "https://x" }));
  assert!(resolve_gitlab_host(Some(&not_object), None).is_err());

  let bad_type = doc(json!({ "gitlab": { "host": 42 } }));
  assert!(resolve_gitlab_host(Some(&bad_type), None).is_err());
}

// ---------------------------------------------------------------------------
// github-like（github / gitee / gitcode）：token 注入三形态
// ---------------------------------------------------------------------------

#[test]
fn github_posts_release_with_bearer_headers() {
  let mock = spawn_mock(|_| (201, r#"{"id":1}"#.to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@github.com:owner/repo.git");
  create_github_like_release(
    Provider::Github,
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
  create_github_like_release(
    Provider::Github,
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

#[test]
fn gitee_injects_token_into_body() {
  let mock = spawn_mock(|_| (201, "{}".to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitee.com:owner/repo.git");
  create_github_like_release(
    Provider::Gitee,
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
fn gitcode_injects_token_into_query() {
  let mock = spawn_mock(|_| (201, "{}".to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@gitcode.com:owner/repo.git");
  create_github_like_release(
    Provider::Gitcode,
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
fn non_2xx_surfaces_server_message() {
  let mock = spawn_mock(|_| (422, r#"{"message":"Validation Failed"}"#.to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@github.com:owner/repo.git");
  let err = create_github_like_release(
    Provider::Github,
    &format!("http://{}", mock.addr),
    "t",
    "1.0.0",
    "",
    &cwd,
  )
  .unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("[422]"), "{msg}");
  assert!(msg.contains("Validation Failed"), "{msg}");
  assert!(msg.contains("Github"), "{msg}");
}

// ---------------------------------------------------------------------------
// gitlab：项目 id 直查 + PRIVATE-TOKEN
// ---------------------------------------------------------------------------

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
  create_gitlab_release(
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
  let err =
    create_gitlab_release(&format!("http://{}", mock.addr), "t", "1.0.0", "", &cwd).unwrap_err();
  assert!(err.to_string().contains("404 Project Not Found"), "{err}");
}
