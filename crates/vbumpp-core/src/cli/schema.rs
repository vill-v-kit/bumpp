//! schema 子命令：配置形状 JSON Schema 的导出通路。无 `--write`
//! 时 stdout 打印纯 JSON（管道重定向友好——CI 再生与管道共用，不混入任何其他
//! 打印）；`--write` 落盘，`--project`（默认）写 `./vbumpprc.schema.json`、
//! `--global` 写 `~/.vbumpp/schema.json`（`VBUMPP_HOME` 生效），落点按显示
//! 路径规范打印。

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde_json::to_string_pretty;

use super::output::{error_line, success_line};
use super::parse::SchemaArgs;
use super::RunEnv;
use crate::config::config_schema;
use crate::display;
use crate::home::vbumpp_home;

pub(super) fn schema_command(
  args: &SchemaArgs,
  env: &RunEnv,
  out: &mut impl Write,
  err: &mut impl Write,
) -> i32 {
  let text = to_string_pretty(&config_schema()).expect("schema serialization cannot fail");
  // stdout 通路：纯 JSON、零其他打印（JSON 解析器直接消费）
  if !args.write {
    let _ = writeln!(out, "{text}");
    return 0;
  }
  let cwd = match env.cwd {
    Some(path) => path.to_path_buf(),
    None => match env::current_dir() {
      Ok(cwd) => cwd,
      Err(e) => {
        error_line(err, &format!("cannot resolve current directory: {e}"));
        return 1;
      }
    },
  };
  let target = if args.global {
    // 注入优先于环境解析（同 store 先例）；home 不可解析即报错
    let Some(home) = env.home.map(PathBuf::from).or_else(vbumpp_home) else {
      error_line(err, "cannot resolve the global vbumpp home directory");
      return 1;
    };
    home.join("schema.json")
  } else {
    cwd.join("vbumpprc.schema.json")
  };
  // 全局落点的父目录可能尚未创建（~/.vbumpp 首次使用）；项目级父目录恒存在
  if let Some(parent) = target.parent() {
    if let Err(e) = fs::create_dir_all(parent) {
      error_line(
        err,
        &format!(
          "failed to create directory {}: {e}",
          display::path(&cwd, parent)
        ),
      );
      return 1;
    }
  }
  match fs::write(&target, format!("{text}\n")) {
    Ok(()) => {
      success_line(
        out,
        &format!("schema written to {}", display::path(&cwd, &target)),
      );
      0
    }
    Err(e) => {
      error_line(
        err,
        &format!(
          "failed to write schema file {}: {e}",
          display::path(&cwd, &target)
        ),
      );
      1
    }
  }
}
