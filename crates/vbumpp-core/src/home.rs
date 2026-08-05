//! 全局配置目录（`~/.vbumpp/`）解析——Token 存储与全局配置文件共享的家（ADR-0015）。
//! 优先级：`VBUMPP_HOME` > `~/.vbumpp`；不引入 XDG。

use std::env;
use std::path::PathBuf;

/// 全局配置目录：`VBUMPP_HOME` 环境变量覆盖，否则 `~/.vbumpp`。
/// home 目录不可解析时返回 None（调用方按自身语义处理：token 报错、全局配置层软跳过）
pub fn vbumpp_home() -> Option<PathBuf> {
  if let Some(custom) = env::var_os("VBUMPP_HOME") {
    if !custom.is_empty() {
      return Some(PathBuf::from(custom));
    }
  }
  dirs::home_dir().map(|h| h.join(".vbumpp"))
}
