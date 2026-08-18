//! 路径解析优先级：`VBUMPP_TOKEN_STORE` > `VBUMPP_HOME` > `~/.vbumpp`。
//! env 修改为进程全局——本文件仅此一个测试函数，隔离并发竞态。

use std::path::PathBuf;

use vbumpp_core::home::vbumpp_home;
use vbumpp_core::token::store_path;

#[test]
fn env_precedence() {
  let saved_home = std::env::var_os("VBUMPP_HOME");
  let saved_store = std::env::var_os("VBUMPP_TOKEN_STORE");

  std::env::remove_var("VBUMPP_TOKEN_STORE");
  std::env::set_var("VBUMPP_HOME", "/tmp/vbumpp-home-x");
  assert_eq!(vbumpp_home().unwrap(), PathBuf::from("/tmp/vbumpp-home-x"));
  assert_eq!(
    store_path().unwrap(),
    PathBuf::from("/tmp/vbumpp-home-x/tokens.bin"),
    "默认存储路径 = 全局配置目录/tokens.bin"
  );

  std::env::set_var("VBUMPP_TOKEN_STORE", "/tmp/custom-store-y.bin");
  assert_eq!(
    store_path().unwrap(),
    PathBuf::from("/tmp/custom-store-y.bin"),
    "VBUMPP_TOKEN_STORE 优先于 VBUMPP_HOME"
  );

  // 还原环境
  match saved_home {
    Some(v) => std::env::set_var("VBUMPP_HOME", v),
    None => std::env::remove_var("VBUMPP_HOME"),
  }
  match saved_store {
    Some(v) => std::env::set_var("VBUMPP_TOKEN_STORE", v),
    None => std::env::remove_var("VBUMPP_TOKEN_STORE"),
  }
}
