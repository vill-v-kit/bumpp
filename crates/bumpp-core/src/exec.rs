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
  let display = format!("{program} {}", args.join(" "));
  let output = spawn(program, args, cwd)?;
  if output.status.success() {
    replay(&output);
    Ok(output)
  } else {
    Err(ExecError::Failed {
      message: format!(
        "{display} 退出码 {}：\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).trim()
      ),
    })
  }
}

fn spawn(program: &str, args: &[String], cwd: &Path) -> Result<Output, ExecError> {
  Command::new(program)
    .args(args)
    .current_dir(cwd)
    .output()
    .map_err(|e| ExecError::Spawn {
      message: format!("执行 {program} {} 失败：{e}", args.join(" ")),
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
