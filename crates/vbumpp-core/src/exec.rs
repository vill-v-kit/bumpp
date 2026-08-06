//! shell-out 执行原语：捕获 stdout/stderr（供错误信息携带），成功后回放到父进程
//! 标准流（近似上游 stdio: inherit 的可见性）。

use std::error::Error;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug)]
pub enum ExecError {
  Spawn { message: String },
  Io { message: String },
  Failed { message: String },
}

impl fmt::Display for ExecError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Spawn { message } | Self::Io { message } | Self::Failed { message } => {
        f.write_str(message)
      }
    }
  }
}

impl Error for ExecError {}

/// 执行命令并捕获输出；非零退出返回含 stderr 的错误（不回放，stderr 由错误携带）
pub fn run(program: &str, args: &[String], cwd: &Path) -> Result<Output, ExecError> {
  let output = capture(program, args, cwd)?;
  replay(&output);
  Ok(output)
}

/// 只读查询的静默捕获：与 `run` 同错误语义，但成功不回放——
/// 查询输出是返回值而非给用户看的进度（git describe / log / rev-parse 等）
pub fn capture(program: &str, args: &[String], cwd: &Path) -> Result<Output, ExecError> {
  let display = format!("{program} {}", args.join(" "));
  let output = spawn(program, args, cwd)?;
  if output.status.success() {
    Ok(output)
  } else {
    Err(ExecError::Failed {
      message: format!(
        "{display} exited with code {}:\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).trim()
      ),
    })
  }
}

/// 带 stdin 供给的捕获（`git check-ignore --stdin` 等批量查询，COL-61）：
/// 仅 spawn / IO 失败报错——退出码语义归调用方（check-ignore 的
/// 0=有命中 / 1=无命中、ls-files 的 0=成功 之类各不相同）
pub fn capture_with_stdin(
  program: &str,
  args: &[String],
  stdin: &[u8],
  cwd: &Path,
) -> Result<Output, ExecError> {
  let mut child = Command::new(program)
    .args(args)
    .current_dir(cwd)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .map_err(|e| ExecError::Spawn {
      message: format!("failed to execute {program} {}: {e}", args.join(" ")),
    })?;
  // stdin 写入独立线程（std 文档范式）：child 边读边产出时双向管道缓冲不互锁；
  // 写入失败（child 早退断管）吞掉——退出码由调用方裁决
  let mut pipe = child.stdin.take().expect("stdin is piped");
  let input = stdin.to_owned();
  let writer = std::thread::spawn(move || pipe.write_all(&input));
  let output = child.wait_with_output().map_err(|e| ExecError::Io {
    message: format!("failed to read output of {program}: {e}"),
  })?;
  let _ = writer.join();
  Ok(output)
}

fn spawn(program: &str, args: &[String], cwd: &Path) -> Result<Output, ExecError> {
  Command::new(program)
    .args(args)
    .current_dir(cwd)
    .output()
    .map_err(|e| ExecError::Spawn {
      message: format!("failed to execute {program} {}: {e}", args.join(" ")),
    })
}

fn replay(output: &Output) {
  if !output.stdout.is_empty() {
    let _ = std::io::stdout().write_all(&output.stdout);
  }
  if !output.stderr.is_empty() {
    let _ = std::io::stderr().write_all(&output.stderr);
  }
}
