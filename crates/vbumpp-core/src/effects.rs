//! 效应边界：bump / release 流水线的全部副作用收口为 `Effects` 四个原语——
//! 文件写盘、子进程执行（git / scripts / install / execute）、平台 HTTP
//! 收发。计算与判定留在流水线本体，副作用经注入的 `Effects` 实现执行：
//! 生产路径一律为 `RealEffects`（语义等同 `std::fs::write` / `exec::run` /
//! ureq 直调，真实行为逐字节不变）；预演路径（--dry-run）以记录型实现骑
//! 同一条流水线——预演与执行同路，预览保真由结构保证而非复制逻辑。
//!
//! 错误契约：`run` 的错误类型与 `exec::run` 一致（调用方原样传播）；
//! `http_*` 仅传输层失败返回 Err（ureq 错误描述，状态码裁决归调用方）。

use std::fs;
use std::io;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use serde_json::Value;

use crate::exec::{run, ExecError};

/// HTTP 响应（传输层产物）：状态码 + 响应体全文（读取失败按空串回落——
/// 对齐原 check_status 的 `unwrap_or_default`，非 2xx 报错提取不受影响）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
  pub status: u16,
  pub body: String,
}

/// 统一效应边界：流水线副作用的唯一通道
pub trait Effects {
  /// 文件写盘（语义同 `std::fs::write`；错误消息格式化归调用方）
  fn write_file(&self, path: &Path, content: &str) -> io::Result<()>;
  /// 子进程执行（语义同 `exec::run`：捕获 + 回放、非零退出报错）
  fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<(), ExecError>;
  /// HTTP GET（Err 仅传输层失败——ureq 错误描述；非 2xx 由调用方裁决）
  fn http_get(&self, url: &str, headers: &[(&str, String)]) -> Result<HttpResponse, String>;
  /// HTTP POST JSON（错误契约同 `http_get`）
  fn http_post_json(
    &self,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
  ) -> Result<HttpResponse, String>;
}

/// 生产效应实现：真实写盘 / spawn / ureq 收发
pub struct RealEffects;

impl Effects for RealEffects {
  fn write_file(&self, path: &Path, content: &str) -> io::Result<()> {
    fs::write(path, content)
  }

  fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<(), ExecError> {
    run(program, args, cwd).map(|_| ())
  }

  fn http_get(&self, url: &str, headers: &[(&str, String)]) -> Result<HttpResponse, String> {
    let mut request = AGENT.get(url);
    for (name, value) in headers {
      request = request.header(*name, value);
    }
    let mut resp = request.call().map_err(|e| e.to_string())?;
    read_response(&mut resp)
  }

  fn http_post_json(
    &self,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
  ) -> Result<HttpResponse, String> {
    let mut request = AGENT.post(url);
    for (name, value) in headers {
      request = request.header(*name, value);
    }
    let mut resp = request.send_json(body).map_err(|e| e.to_string())?;
    read_response(&mut resp)
  }
}

/// 共享 Agent（进程级，ureq 内部 Arc 跨线程共享）：30s 全局超时；状态码不当
/// 异常（手动检查以提取服务端错误信息）。连接池共享保留原 gitlab GET+POST
/// 同 agent 的行为
static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
  ureq::Agent::config_builder()
    .timeout_global(Some(Duration::from_secs(30)))
    .http_status_as_error(false)
    .build()
    .into()
});

fn read_response(resp: &mut ureq::http::Response<ureq::Body>) -> Result<HttpResponse, String> {
  let status = resp.status().as_u16();
  let body = resp.body_mut().read_to_string().unwrap_or_default();
  Ok(HttpResponse { status, body })
}
