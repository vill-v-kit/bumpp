//! JsManifestPlugin 行为矩阵——对齐上游 bumpp v11 updateManifestFile（ADR-0004 纯迁移）。

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use version_files::{update_file, UpdateOutcome};

fn bump(dir: &TempDir, name: &str) -> UpdateOutcome {
  update_file(Path::new(name), &dir.path().join(name), "1.0.0", "2.0.0").unwrap()
}

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn read(dir: &TempDir, name: &str) -> String {
  fs::read_to_string(dir.path().join(name)).unwrap()
}

#[test]
fn manifest_update_preserves_formatting() {
  let dir = TempDir::new().unwrap();
  let original = "{\n    \"name\": \"demo\",\n    \"version\": \"1.0.0\",\n    \"description\": \"d\",\n    \"private\": true\n}\n";
  write(&dir, "package.json", original);
  assert_eq!(bump(&dir, "package.json"), UpdateOutcome::Updated);
  // 仅 version 值被替换，缩进（4 空格）、字段序、尾部换行全部保留
  assert_eq!(
    read(&dir, "package.json"),
    original.replace("\"version\": \"1.0.0\"", "\"version\": \"2.0.0\"")
  );
}

#[test]
fn manifest_update_preserves_jsonc_comments() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "deno.jsonc",
    "{\n  // 注释保留\n  \"version\": \"1.0.0\", // trailing\n}\n",
  );
  assert_eq!(bump(&dir, "deno.jsonc"), UpdateOutcome::Updated);
  assert_eq!(
    read(&dir, "deno.jsonc"),
    "{\n  // 注释保留\n  \"version\": \"2.0.0\", // trailing\n}\n"
  );
}

#[test]
fn package_lock_updates_nested_packages_version() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "package-lock.json",
    "{\n  \"name\": \"demo\",\n  \"version\": \"1.0.0\",\n  \"lockfileVersion\": 3,\n  \"packages\": {\n    \"\": {\n      \"name\": \"demo\",\n      \"version\": \"1.0.0\"\n    },\n    \"node_modules/dep\": {\n      \"version\": \"1.0.0\"\n    }\n  }\n}\n",
  );
  assert_eq!(bump(&dir, "package-lock.json"), UpdateOutcome::Updated);
  // 顶层与 packages[""] 更新，node_modules/dep 的 version 不动（上游只改这两处）
  assert_eq!(
    read(&dir, "package-lock.json"),
    "{\n  \"name\": \"demo\",\n  \"version\": \"2.0.0\",\n  \"lockfileVersion\": 3,\n  \"packages\": {\n    \"\": {\n      \"name\": \"demo\",\n      \"version\": \"2.0.0\"\n    },\n    \"node_modules/dep\": {\n      \"version\": \"1.0.0\"\n    }\n  }\n}\n"
  );
}

#[test]
fn package_lock_without_nested_version_only_updates_top() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "package-lock.json",
    "{\n  \"version\": \"1.0.0\",\n  \"packages\": {}\n}\n",
  );
  assert_eq!(bump(&dir, "package-lock.json"), UpdateOutcome::Updated);
  assert_eq!(
    read(&dir, "package-lock.json"),
    "{\n  \"version\": \"2.0.0\",\n  \"packages\": {}\n}\n"
  );
}

#[test]
fn manifest_with_same_version_is_skipped_untouched() {
  let dir = TempDir::new().unwrap();
  let original = "{\n  \"version\": \"2.0.0\"\n}\n";
  write(&dir, "package.json", original);
  assert_eq!(bump(&dir, "package.json"), UpdateOutcome::Skipped);
  assert_eq!(read(&dir, "package.json"), original, "未修改时不应重写文件");
}

#[test]
fn manifest_without_version_is_skipped() {
  let dir = TempDir::new().unwrap();
  write(&dir, "package.json", "{\n  \"name\": \"demo\"\n}\n");
  assert_eq!(bump(&dir, "package.json"), UpdateOutcome::Skipped);
}

#[test]
fn manifest_with_null_version_is_skipped() {
  let dir = TempDir::new().unwrap();
  write(&dir, "bower.json", "{\n  \"version\": null\n}\n");
  assert_eq!(bump(&dir, "bower.json"), UpdateOutcome::Skipped);
}

#[test]
fn non_manifest_json_is_skipped() {
  let dir = TempDir::new().unwrap();
  // version 为数字 → 不满足 isManifest（name/version/description 须为可选字符串）
  write(&dir, "package.json", "{\n  \"version\": 42\n}\n");
  assert_eq!(bump(&dir, "package.json"), UpdateOutcome::Skipped);
}

#[test]
fn invalid_manifest_json_is_skipped() {
  let dir = TempDir::new().unwrap();
  // 上游 jsonc.parse 容错：坏 JSON 的 manifest 按 skip 处理
  write(&dir, "package.json", "{ not json");
  assert_eq!(bump(&dir, "package.json"), UpdateOutcome::Skipped);
}

#[test]
fn manifest_basename_matching_is_case_insensitive() {
  let dir = TempDir::new().unwrap();
  write(&dir, "PACKAGE.JSON", "{\n  \"version\": \"1.0.0\"\n}\n");
  assert_eq!(bump(&dir, "PACKAGE.JSON"), UpdateOutcome::Updated);
  assert_eq!(
    read(&dir, "PACKAGE.JSON"),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
}
