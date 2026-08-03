//! 集成测试共享辅助。
#![allow(dead_code)] // 各测试二进制按需取用——逐二进制编译必有未用项

use std::path::Path;
use std::process::Command;
use std::sync::Once;

static ISOLATE: Once = Once::new();

/// 把全局配置目录指向不存在的路径，使走 `read_document`（env 解析全局层）的
/// 测试不受宿主机真实 `~/.vbumpp/config.*` 影响。进程内所有线程 set 同值，
/// Once 保证单次执行，无并发竞态。
pub fn isolate_global_home() {
  ISOLATE.call_once(|| {
    std::env::set_var("VBUMPP_HOME", "/nonexistent/vbumpp-global-home");
  });
}

/// 临时 git 仓库命令执行：断言成功并返回 trim 后的 stdout
pub fn git(dir: &Path, args: &[&str]) -> String {
  let output = Command::new("git")
    .args(args)
    .current_dir(dir)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "git {args:?} 失败：{}",
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
