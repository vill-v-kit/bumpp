//! 通用脚本执行（ADR-0011）：配置声明的 shell 命令真实执行、非零退出报错传播。

use std::fs;

use tempfile::TempDir;
use vbumpp_core::scripts::run_script;

#[test]
fn script_runs_via_shell() {
  let dir = TempDir::new().unwrap();
  run_script(dir.path(), "touch ran.txt").unwrap();
  assert!(dir.path().join("ran.txt").exists(), "脚本应真实执行");
}

#[test]
fn script_supports_shell_features() {
  // 经 shell 执行：重定向等 shell 特性可用
  let dir = TempDir::new().unwrap();
  run_script(dir.path(), "echo hi > out.txt").unwrap();
  assert_eq!(
    fs::read_to_string(dir.path().join("out.txt"))
      .unwrap()
      .trim(),
    "hi"
  );
}

#[test]
fn script_failure_propagates() {
  // ADR-0011：配置声明的钩子非零退出即报错（发版不得静默继续，
  // 对齐 ADR-0003 失败即报错精神，有意偏离上游 npm scripts 不传播的 parity）
  let dir = TempDir::new().unwrap();
  let err = run_script(dir.path(), "exit 1").unwrap_err();
  assert!(err.to_string().contains("exit 1"), "错误应含命令：{err}");
}

#[test]
fn script_runs_in_cwd() {
  let dir = TempDir::new().unwrap();
  fs::create_dir(dir.path().join("sub")).unwrap();
  run_script(&dir.path().join("sub"), "touch where.txt").unwrap();
  assert!(dir.path().join("sub/where.txt").exists());
}
