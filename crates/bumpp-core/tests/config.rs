//! loadBumpConfig 合并矩阵——对齐上游 antfu/bumpp v11 的浅展开语义：
//! `bumpConfigDefaults` ← 配置文件 ← overrides（undefined/null 剥离）。

use std::fs;

use bumpp_core::config::{bump_config_defaults, load_bump_config, LoadConfigError};
use serde_json::{json, Map, Value};
use tempfile::TempDir;

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn overrides(v: Value) -> Option<Map<String, Value>> {
  Some(v.as_object().unwrap().clone())
}

fn load(dir: &TempDir, o: Option<Map<String, Value>>) -> Map<String, Value> {
  load_bump_config(o, dir.path()).unwrap()
}

#[test]
fn defaults_shape_matches_upstream() {
  let d = bump_config_defaults();
  assert_eq!(d["commit"], true);
  assert_eq!(d["push"], true);
  assert_eq!(d["tag"], true);
  assert_eq!(d["sign"], false);
  assert_eq!(d["install"], false);
  assert_eq!(d["recursive"], false);
  assert_eq!(d["noVerify"], false);
  assert_eq!(d["confirm"], true);
  assert_eq!(d["ignoreScripts"], false);
  assert_eq!(d["all"], false);
  assert_eq!(d["noGitCheck"], true);
  assert_eq!(d["files"], json!([]));
}

#[test]
fn no_config_file_returns_defaults() {
  let dir = TempDir::new().unwrap();
  let merged = load(&dir, None);
  for (k, v) in bump_config_defaults() {
    assert_eq!(&merged[&k], &v, "key {k}");
  }
}

#[test]
fn overrides_apply_without_config_file() {
  let dir = TempDir::new().unwrap();
  let merged = load(
    &dir,
    overrides(json!({ "commit": false, "files": ["a.json"] })),
  );
  assert_eq!(merged["commit"], false);
  assert_eq!(merged["files"], json!(["a.json"]));
  assert_eq!(merged["tag"], true, "其余键回落到 defaults");
}

#[test]
fn bump_config_json_is_silently_ignored() {
  let dir = TempDir::new().unwrap();
  // 旧名不探测（ADR-0013）：内容与默认冲突也不读、不报错
  write(&dir, "bump.config.json", r#"{ "tag": false }"#);
  let merged = load(&dir, None);
  assert_eq!(merged["tag"], true, "旧名 bump.config.json 应静默失效");
}

#[test]
fn old_script_esconf_and_changelogen_configs_are_silently_ignored() {
  // ADR-0013：bump.config 脚本系 / vbumpp.config esconf 系 / changelogen 系
  // 旧配置文件一律不探测、不读取、不报错
  for (name, content) in [
    ("bump.config.ts", "export default { tag: false }"),
    ("bump.config.js", "module.exports = { tag: false }"),
    (
      "vbumpp.config.ts",
      "export default { bumpp: { tag: false } }",
    ),
    ("vbumpp.json", r#"{ "bumpp": { "tag": false } }"#),
    ("changelog.config.ts", "export default { output: 'X.md' }"),
  ] {
    let dir = TempDir::new().unwrap();
    write(&dir, name, content);
    let merged = load(&dir, None);
    assert_eq!(merged["tag"], true, "{name} 应静默失效");
  }
}

#[test]
fn json_config_beats_defaults() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.json",
    r#"{ "tag": false, "preid": "beta" }"#,
  );
  let merged = load(&dir, None);
  assert_eq!(merged["tag"], false);
  assert_eq!(merged["preid"], "beta");
  assert_eq!(merged["commit"], true, "未覆盖的键回落到 defaults");
}

#[test]
fn overrides_beat_config_file() {
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", r#"{ "tag": false, "push": false }"#);
  let merged = load(&dir, overrides(json!({ "tag": true })));
  assert_eq!(merged["tag"], true, "overrides 最高优先");
  assert_eq!(merged["push"], false, "文件配置次之");
}

#[test]
fn null_overrides_are_stripped_like_undefined() {
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", r#"{ "tag": false }"#);
  // 经 napi 传入时 JS undefined 会序列化为 null，对齐上游 `v !== void 0` 剥离语义
  let merged = load(&dir, overrides(json!({ "tag": null })));
  assert_eq!(merged["tag"], false, "null 不应覆盖文件配置");
}

#[test]
fn arrays_are_replaced_not_concatenated() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.json",
    r#"{ "files": ["from-config.json"] }"#,
  );
  let merged = load(&dir, overrides(json!({ "files": ["from-overrides.json"] })));
  assert_eq!(
    merged["files"],
    json!(["from-overrides.json"]),
    "上游浅展开语义：数组整体替换而非拼接"
  );
}

#[test]
fn nested_objects_are_replaced_not_deep_merged() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.json",
    r#"{ "commit": { "message": "from config" } }"#,
  );
  let merged = load(&dir, overrides(json!({ "commit": { "all": true } })));
  assert_eq!(
    merged["commit"],
    json!({ "all": true }),
    "上游浅展开语义：嵌套对象整体替换"
  );
}

#[test]
fn custom_version_in_file_errors() {
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", r#"{ "customVersion": "1.2.3" }"#);
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(
        message.contains("customVersion"),
        "应指出被移除的选项：{message}"
      );
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn config_file_path_loads_exact_file() {
  let dir = TempDir::new().unwrap();
  write(&dir, "my-custom.json", r#"{ "push": false }"#);
  let merged = load(
    &dir,
    overrides(json!({ "configFilePath": "my-custom.json" })),
  );
  assert_eq!(merged["push"], false);
}

#[test]
fn config_file_path_to_ts_errors() {
  let dir = TempDir::new().unwrap();
  write(&dir, "custom.ts", "export default {}");
  let err = load_bump_config(
    overrides(json!({ "configFilePath": "custom.ts" })),
    dir.path(),
  )
  .unwrap_err();
  assert!(matches!(err, LoadConfigError::UnsupportedConfig { .. }));
}

#[test]
fn malformed_json_reports_path() {
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", "{ not json");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::Parse { message } => {
      assert!(
        message.contains(".vbumpprc.json"),
        "报错应含文件路径：{message}"
      );
    }
    other => panic!("应为 Parse，实际 {other:?}"),
  }
}

/// 插件底座链聚合的 recursive 模式表（node 8 清单 + Cargo.toml，链序 Node → Cargo）——
/// 独立事实源：字面量断言，不经被测代码重算
const EXPECTED_MANIFEST_GLOBS: [&str; 9] = [
  "**/package.json",
  "**/package-lock.json",
  "**/bower.json",
  "**/component.json",
  "**/jsr.json",
  "**/jsr.jsonc",
  "**/deno.json",
  "**/deno.jsonc",
  "**/Cargo.toml",
];

#[test]
fn recursive_in_overrides_expands_manifest_globs() {
  let dir = TempDir::new().unwrap();
  let merged = load(&dir, overrides(json!({ "recursive": true })));
  assert_eq!(merged["files"], json!(EXPECTED_MANIFEST_GLOBS));
  assert_eq!(merged["recursive"], false, "展开后 recursive 置 false");
}

#[test]
fn recursive_in_config_file_expands_manifest_globs() {
  // merged 语义（ADR-0013）：recursive 来自配置文件同样展开
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", r#"{ "recursive": true }"#);
  let merged = load(&dir, None);
  assert_eq!(merged["files"], json!(EXPECTED_MANIFEST_GLOBS));
  assert_eq!(merged["recursive"], false);
}

#[test]
fn recursive_expansion_dedupes_preserving_order() {
  let dir = TempDir::new().unwrap();
  let merged = load(
    &dir,
    overrides(json!({ "files": ["README.md", "**/package.json"], "recursive": true })),
  );
  let mut expected = vec!["README.md", "**/package.json"];
  expected.extend(EXPECTED_MANIFEST_GLOBS.into_iter().skip(1));
  assert_eq!(
    merged["files"],
    json!(expected),
    "既有 files 保留在前，重复项不二次出现"
  );
}

#[test]
fn recursive_absent_leaves_files_untouched() {
  let dir = TempDir::new().unwrap();
  let merged = load(&dir, overrides(json!({ "files": ["a.json"] })));
  assert_eq!(merged["files"], json!(["a.json"]));
  assert_eq!(merged["recursive"], false);
}

#[test]
fn files_are_deduped_without_recursive() {
  // 原 JS 后处理的无条件去重语义随加载器一并迁移（ADR-0013）
  let dir = TempDir::new().unwrap();
  let merged = load(
    &dir,
    overrides(json!({ "files": ["a.json", "b.json", "a.json"] })),
  );
  assert_eq!(merged["files"], json!(["a.json", "b.json"]));
}
