//! 静态链分发：按 matches 顺序命中通道，TextPlugin 兜底（ADR-0004）。

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use version_files::{update_file, UpdateOutcome};

#[test]
fn manifest_basename_wins_over_text_fallback() {
  let dir = TempDir::new().unwrap();
  // 坏 JSON 的 package.json：若误入 Text 通道会被当成文本替换，
  // 走 JsManifest 通道则按上游容错规则 skip
  fs::write(dir.path().join("package.json"), "{ version 1.0.0").unwrap();
  let outcome = update_file(
    Path::new("package.json"),
    &dir.path().join("package.json"),
    "1.0.0",
    "2.0.0",
  )
  .unwrap();
  assert_eq!(outcome, UpdateOutcome::Skipped);
  assert_eq!(
    fs::read_to_string(dir.path().join("package.json")).unwrap(),
    "{ version 1.0.0",
    "manifest 通道 skip 不得改写文件"
  );
}

#[test]
fn unknown_basename_falls_through_to_text() {
  let dir = TempDir::new().unwrap();
  fs::write(dir.path().join("VERSION.txt"), "version 1.0.0\n").unwrap();
  let outcome = update_file(
    Path::new("VERSION.txt"),
    &dir.path().join("VERSION.txt"),
    "1.0.0",
    "2.0.0",
  )
  .unwrap();
  assert_eq!(outcome, UpdateOutcome::Updated);
  assert_eq!(
    fs::read_to_string(dir.path().join("VERSION.txt")).unwrap(),
    "version 2.0.0\n"
  );
}

#[test]
fn matches_uses_basename_not_full_path() {
  let dir = TempDir::new().unwrap();
  fs::create_dir_all(dir.path().join("sub/deep")).unwrap();
  fs::write(
    dir.path().join("sub/deep/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  let outcome = update_file(
    Path::new("sub/deep/package.json"),
    &dir.path().join("sub/deep/package.json"),
    "1.0.0",
    "2.0.0",
  )
  .unwrap();
  assert_eq!(outcome, UpdateOutcome::Updated);
  assert_eq!(
    fs::read_to_string(dir.path().join("sub/deep/package.json")).unwrap(),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
}

#[test]
fn io_error_message_uses_rel_path() {
  let dir = TempDir::new().unwrap();
  // 错误消息沿用迁移前文案：用户清单中的相对路径（而非内部绝对路径）
  let err = update_file(
    Path::new("sub/ghost.txt"),
    &dir.path().join("sub/ghost.txt"),
    "1.0.0",
    "2.0.0",
  )
  .unwrap_err();
  let message = err.to_string();
  assert!(
    message.starts_with("读取 sub/ghost.txt 失败："),
    "错误消息应以相对路径开头，实际：{message}"
  );
}

#[test]
fn cargo_toml_wins_over_text_fallback() {
  let dir = TempDir::new().unwrap();
  // version.workspace = true 的成员：走 TOML 通道按 ADR-0003 跳过；
  // 若误入 Text 通道，注释里的 1.0.0 会被正则替换
  fs::write(
    dir.path().join("Cargo.toml"),
    "[package]\nname = \"demo\"\nversion.workspace = true\n# 提到 1.0.0 也不应被文本替换\n",
  )
  .unwrap();
  let outcome = update_file(
    Path::new("Cargo.toml"),
    &dir.path().join("Cargo.toml"),
    "1.0.0",
    "2.0.0",
  )
  .unwrap();
  assert_eq!(outcome, UpdateOutcome::Skipped);
  assert_eq!(
    fs::read_to_string(dir.path().join("Cargo.toml")).unwrap(),
    "[package]\nname = \"demo\"\nversion.workspace = true\n# 提到 1.0.0 也不应被文本替换\n",
    "TOML 通道 skip 不得改写文件"
  );
}

#[test]
fn cargo_toml_basename_matching_is_case_insensitive() {
  let dir = TempDir::new().unwrap();
  fs::write(
    dir.path().join("CARGO.TOML"),
    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  )
  .unwrap();
  let outcome = update_file(
    Path::new("CARGO.TOML"),
    &dir.path().join("CARGO.TOML"),
    "1.0.0",
    "2.0.0",
  )
  .unwrap();
  assert_eq!(outcome, UpdateOutcome::Updated);
  assert_eq!(
    fs::read_to_string(dir.path().join("CARGO.TOML")).unwrap(),
    "[package]\nname = \"demo\"\nversion = \"2.0.0\"\n"
  );
}
