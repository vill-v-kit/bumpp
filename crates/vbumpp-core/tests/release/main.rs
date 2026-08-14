//! 平台 Release（ADR-0014）：token 解析链 / gitlab.host schema 纯测 +
//! 四家 API 的 mock server 行为测试（零外部依赖的手写 HTTP mock）。
//! 目录镜像 src/release/（ADR-0014）：共享 mock 线束与共享层测试在根部，
//! token 链与各 provider 行为各一文件。

#[path = "../common.rs"]
mod common;

mod gitcode;
mod gitee;
mod github;
mod gitlab;
mod token;
mod token_source;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Mutex, MutexGuard};

use tempfile::TempDir;

// ---------------------------------------------------------------------------
// 真实入口（create_release）用例的 env 线束：token 链走真实环境解析，
// env 修改为进程全局——ENV_LOCK 串行 + 入场净化 + 存储指向临时位置
// ---------------------------------------------------------------------------

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 入场串行 + 净化：清掉全部 provider token 环境变量，token 存储指向
/// 临时位置，全局配置目录隔离（read_document 全局层不受宿主机影响）
fn sanitized_token_env(store: &std::path::Path) -> MutexGuard<'static, ()> {
  let guard = ENV_LOCK.lock().unwrap();
  for key in common::PROVIDER_TOKEN_ENV_VARS {
    std::env::remove_var(key);
  }
  std::env::set_var("VBUMPP_TOKEN_STORE", store);
  common::isolate_global_home();
  guard
}

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

  fn json(&self) -> serde_json::Value {
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
// 共享层：非 2xx 提取服务端 message（经 github 注入缝走查 check_status）
// ---------------------------------------------------------------------------

#[test]
fn non_2xx_surfaces_server_message() {
  let mock = spawn_mock(|_| (422, r#"{"message":"Validation Failed"}"#.to_owned()));
  let dir = TempDir::new().unwrap();
  let cwd = git_repo(&dir, "git@github.com:owner/repo.git");
  let err = vbumpp_core::release::github::create_with_base(
    &format!("http://{}", mock.addr),
    "ghp-secret-token",
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
