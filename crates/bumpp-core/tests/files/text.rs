//! TextPlugin 行为矩阵——对齐上游 bumpp v11 updateTextFile（ADR-0007 纯迁移）。

use std::fs;
use std::path::Path;

use bumpp_core::files::{dispatch_file as update_file, UpdateOutcome};
use tempfile::TempDir;

fn bump(dir: &TempDir, name: &str, current: &str, new: &str) -> UpdateOutcome {
  update_file(Path::new(name), &dir.path().join(name), current, new).unwrap()
}

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn read(dir: &TempDir, name: &str) -> String {
  fs::read_to_string(dir.path().join(name)).unwrap()
}

#[test]
fn text_file_replaces_all_occurrences() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "CHANGELOG.md",
    "## v1.0.0\n\nChanges since 1.0.0:\n- pin 11.0.0 stays\n- foo1.0.0bar stays\n- 1.0.0-beta.1 context\n",
  );
  assert_eq!(
    bump(&dir, "CHANGELOG.md", "1.0.0", "2.0.0"),
    UpdateOutcome::Updated
  );
  assert_eq!(
    read(&dir, "CHANGELOG.md"),
    "## v2.0.0\n\nChanges since 2.0.0:\n- pin 11.0.0 stays\n- foo1.0.0bar stays\n- 2.0.0-beta.1 context\n"
  );
}

#[test]
fn text_file_word_boundary_is_ascii_like_js() {
  let dir = TempDir::new().unwrap();
  // JS \b 是 ASCII 语义：CJK 字符是非 word 字符，故 "版本1.0.0" 中 1 前存在边界
  write(&dir, "notes.txt", "版本1.0.0发布\n");
  assert_eq!(
    bump(&dir, "notes.txt", "1.0.0", "2.0.0"),
    UpdateOutcome::Updated
  );
  assert_eq!(read(&dir, "notes.txt"), "版本2.0.0发布\n");
}

#[test]
fn text_file_without_current_version_is_skipped() {
  let dir = TempDir::new().unwrap();
  write(&dir, "README.md", "# demo\n");
  assert_eq!(
    bump(&dir, "README.md", "1.0.0", "2.0.0"),
    UpdateOutcome::Skipped
  );
}

#[test]
fn prerelease_current_version_in_text() {
  let dir = TempDir::new().unwrap();
  write(&dir, "a.txt", "now at 1.0.0-beta.1!\n");
  assert_eq!(
    bump(&dir, "a.txt", "1.0.0-beta.1", "1.0.0-beta.2"),
    UpdateOutcome::Updated
  );
  assert_eq!(read(&dir, "a.txt"), "now at 1.0.0-beta.2!\n");
}
