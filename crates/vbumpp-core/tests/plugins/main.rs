//! 插件底座编排矩阵——对齐上游 bumpp v11 updateFiles（事件序列、路径归一、
//! 附带文件补发）；生态能力矩阵按 src/plugins/ 能力子目录镜像（ADR-0007）。

use std::fs;

use tempfile::TempDir;
use vbumpp_core::plugins::update_files;
use vbumpp_core::progress::ProgressEvent;

mod dispatch;
mod install;
mod recursive;
mod version;

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn read(dir: &TempDir, name: &str) -> String {
  fs::read_to_string(dir.path().join(name)).unwrap()
}

fn bump(dir: &TempDir, files: &[&str]) -> vbumpp_core::plugins::UpdateFilesOutcome {
  update_files(
    &files.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    dir.path(),
    "1.0.0",
    "2.0.0",
  )
  .unwrap()
}

#[test]
fn manifest_update_preserves_formatting() {
  let dir = TempDir::new().unwrap();
  let original = "{\n    \"name\": \"demo\",\n    \"version\": \"1.0.0\",\n    \"description\": \"d\",\n    \"private\": true\n}\n";
  write(&dir, "package.json", original);
  let outcome = bump(&dir, &["package.json"]);
  assert_eq!(outcome.updated_files().len(), 1);
  assert!(outcome.skipped_files().is_empty());
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
  let outcome = bump(&dir, &["deno.jsonc"]);
  assert_eq!(outcome.updated_files().len(), 1);
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
  let outcome = bump(&dir, &["package-lock.json"]);
  assert_eq!(outcome.updated_files().len(), 1);
  let updated = read(&dir, "package-lock.json");
  // 顶层与 packages[""] 更新，node_modules/dep 的 version 不动（上游只改这两处）
  assert_eq!(
    updated,
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
  let outcome = bump(&dir, &["package-lock.json"]);
  assert_eq!(outcome.updated_files().len(), 1);
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
  let outcome = bump(&dir, &["package.json"]);
  assert!(outcome.updated_files().is_empty());
  assert_eq!(outcome.skipped_files().len(), 1);
  assert_eq!(read(&dir, "package.json"), original, "未修改时不应重写文件");
}

#[test]
fn manifest_without_version_is_skipped() {
  let dir = TempDir::new().unwrap();
  write(&dir, "package.json", "{\n  \"name\": \"demo\"\n}\n");
  write(&dir, "bower.json", "{\n  \"version\": null\n}\n");
  let outcome = bump(&dir, &["package.json", "bower.json"]);
  assert!(outcome.updated_files().is_empty());
  assert_eq!(outcome.skipped_files().len(), 2);
}

#[test]
fn non_manifest_json_is_skipped() {
  let dir = TempDir::new().unwrap();
  // version 为数字 → 不满足 isManifest（name/version/description 须为可选字符串）
  write(&dir, "package.json", "{\n  \"version\": 42\n}\n");
  let outcome = bump(&dir, &["package.json"]);
  assert!(outcome.updated_files().is_empty());
  assert_eq!(outcome.skipped_files().len(), 1);
}

#[test]
fn invalid_manifest_json_is_skipped_and_batch_continues() {
  let dir = TempDir::new().unwrap();
  // 上游 jsonc.parse 容错：坏 JSON 的 manifest 按 skip 处理，批次继续
  write(&dir, "package.json", "{ not json");
  write(&dir, "other.txt", "at 1.0.0\n");
  let outcome = bump(&dir, &["package.json", "other.txt"]);
  assert_eq!(outcome.updated_files().len(), 1);
  assert_eq!(outcome.skipped_files().len(), 1);
  assert_eq!(read(&dir, "other.txt"), "at 2.0.0\n");
}

#[test]
fn events_are_in_processing_order() {
  let dir = TempDir::new().unwrap();
  write(&dir, "b.txt", "at 1.0.0\n");
  write(&dir, "package.json", "{\n  \"version\": \"2.0.0\"\n}\n");
  write(&dir, "a.txt", "at 1.0.0\n");
  let outcome = bump(&dir, &["b.txt", "package.json", "a.txt"]);
  let events: Vec<_> = outcome
    .events()
    .iter()
    .map(|(e, p)| (*e, p.rsplit('/').next().unwrap().to_owned()))
    .collect();
  assert_eq!(
    events,
    vec![
      (ProgressEvent::FileUpdated, "b.txt".to_owned()),
      (ProgressEvent::FileSkipped, "package.json".to_owned()),
      (ProgressEvent::FileUpdated, "a.txt".to_owned()),
    ]
  );
  // 派生视图与上游 state.updatedFiles / skippedFiles 一致
  assert_eq!(outcome.updated_files().len(), 2);
  assert_eq!(outcome.skipped_files().len(), 1);
}

#[test]
fn text_file_replaces_all_occurrences() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "CHANGELOG.md",
    "## v1.0.0\n\nChanges since 1.0.0:\n- pin 11.0.0 stays\n- foo1.0.0bar stays\n- 1.0.0-beta.1 context\n",
  );
  let outcome = bump(&dir, &["CHANGELOG.md"]);
  assert_eq!(outcome.updated_files().len(), 1);
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
  let outcome = bump(&dir, &["notes.txt"]);
  assert_eq!(outcome.updated_files().len(), 1);
  assert_eq!(read(&dir, "notes.txt"), "版本2.0.0发布\n");
}

#[test]
fn text_file_without_current_version_is_skipped() {
  let dir = TempDir::new().unwrap();
  write(&dir, "README.md", "# demo\n");
  let outcome = bump(&dir, &["README.md"]);
  assert!(outcome.updated_files().is_empty());
  assert_eq!(outcome.skipped_files().len(), 1);
}

#[test]
fn nonexistent_file_is_skipped() {
  let dir = TempDir::new().unwrap();
  let outcome = bump(&dir, &["ghost.json"]);
  assert!(outcome.updated_files().is_empty());
  assert_eq!(outcome.skipped_files().len(), 1);
}

#[test]
fn manifest_basename_matching_is_case_insensitive() {
  let dir = TempDir::new().unwrap();
  write(&dir, "PACKAGE.JSON", "{\n  \"version\": \"1.0.0\"\n}\n");
  let outcome = bump(&dir, &["PACKAGE.JSON"]);
  assert_eq!(outcome.updated_files().len(), 1);
  assert_eq!(
    read(&dir, "PACKAGE.JSON"),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
}

#[test]
fn prerelease_current_version_in_text() {
  let dir = TempDir::new().unwrap();
  write(&dir, "a.txt", "now at 1.0.0-beta.1!\n");
  let outcome = update_files(
    &["a.txt".to_string()],
    dir.path(),
    "1.0.0-beta.1",
    "1.0.0-beta.2",
  )
  .unwrap();
  assert_eq!(outcome.updated_files().len(), 1);
  assert_eq!(read(&dir, "a.txt"), "now at 1.0.0-beta.2!\n");
}

#[test]
fn updated_and_skipped_paths_are_absolute() {
  let dir = TempDir::new().unwrap();
  write(&dir, "package.json", "{\n  \"version\": \"1.0.0\"\n}\n");
  let outcome = bump(&dir, &["package.json", "ghost.txt"]);
  assert_eq!(
    outcome.updated_files()[0],
    dir.path().join("package.json").to_string_lossy()
  );
  assert_eq!(
    outcome.skipped_files()[0],
    dir.path().join("ghost.txt").to_string_lossy()
  );
}

#[test]
fn event_paths_are_resolved_like_node_path_resolve() {
  let dir = TempDir::new().unwrap();
  fs::create_dir(dir.path().join("sub")).unwrap();
  write(&dir, "sub/package.json", "{\n  \"version\": \"1.0.0\"\n}\n");
  // 上游用 path.resolve 归一化事件路径：./ 与 .. 段被消除
  let outcome = bump(&dir, &["./sub/../sub/package.json"]);
  assert_eq!(
    outcome.updated_files()[0],
    dir.path().join("sub/package.json").to_string_lossy()
  );
}

#[test]
fn cargo_toml_update_emits_lock_file_updated_event() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "Cargo.toml",
    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  );
  write(
    &dir,
    "Cargo.lock",
    "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  );
  let outcome = bump(&dir, &["Cargo.toml"]);
  // Cargo.toml 带动 Cargo.lock 定向同步（ADR-0003）：附带文件紧随主文件补发
  // FileUpdated，两者都进入 updated_files（git 提交暂存的依据）
  let events: Vec<_> = outcome
    .events()
    .iter()
    .map(|(e, p)| (*e, p.rsplit('/').next().unwrap().to_owned()))
    .collect();
  assert_eq!(
    events,
    vec![
      (ProgressEvent::FileUpdated, "Cargo.toml".to_owned()),
      (ProgressEvent::FileUpdated, "Cargo.lock".to_owned()),
    ]
  );
  assert_eq!(
    outcome.updated_files(),
    vec![
      dir.path().join("Cargo.toml").to_string_lossy().as_ref(),
      dir.path().join("Cargo.lock").to_string_lossy().as_ref(),
    ]
  );
  assert!(outcome.skipped_files().is_empty());
  assert_eq!(
    read(&dir, "Cargo.lock"),
    "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"2.0.0\"\n"
  );
}
