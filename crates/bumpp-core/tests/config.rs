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
fn json_config_beats_defaults() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "bump.config.json",
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
  write(
    &dir,
    "bump.config.json",
    r#"{ "tag": false, "push": false }"#,
  );
  let merged = load(&dir, overrides(json!({ "tag": true })));
  assert_eq!(merged["tag"], true, "overrides 最高优先");
  assert_eq!(merged["push"], false, "文件配置次之");
}

#[test]
fn null_overrides_are_stripped_like_undefined() {
  let dir = TempDir::new().unwrap();
  write(&dir, "bump.config.json", r#"{ "tag": false }"#);
  // 经 napi 传入时 JS undefined 会序列化为 null，对齐上游 `v !== void 0` 剥离语义
  let merged = load(&dir, overrides(json!({ "tag": null })));
  assert_eq!(merged["tag"], false, "null 不应覆盖文件配置");
}

#[test]
fn arrays_are_replaced_not_concatenated() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "bump.config.json",
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
    "bump.config.json",
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
fn ts_config_errors_with_json_only_message() {
  let dir = TempDir::new().unwrap();
  write(&dir, "bump.config.ts", "export default { tag: false }");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(
        message.contains("仅支持 JSON 配置"),
        "应明确仅支持 JSON：{message}"
      );
      assert!(
        message.contains("bump.config.ts"),
        "应指出检测到的文件：{message}"
      );
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn js_variants_also_error() {
  for name in [
    "bump.config.js",
    "bump.config.mjs",
    "bump.config.cjs",
    "bump.config.mts",
    "bump.config.cts",
  ] {
    let dir = TempDir::new().unwrap();
    write(&dir, name, "module.exports = {}");
    let err = load_bump_config(None, dir.path()).unwrap_err();
    assert!(
      matches!(err, LoadConfigError::UnsupportedConfig { .. }),
      "{name} 应报 UnsupportedConfig"
    );
  }
}

#[test]
fn ts_config_errors_even_when_json_present() {
  let dir = TempDir::new().unwrap();
  write(&dir, "bump.config.json", r#"{ "tag": false }"#);
  write(&dir, "bump.config.ts", "export default {}");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  assert!(
    matches!(err, LoadConfigError::UnsupportedConfig { .. }),
    "存在脚本配置即报错，不静默忽略（即使 json 并存）"
  );
}

#[test]
fn custom_version_in_file_errors() {
  let dir = TempDir::new().unwrap();
  write(&dir, "bump.config.json", r#"{ "customVersion": "1.2.3" }"#);
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
  write(&dir, "bump.config.json", "{ not json");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::Parse { message } => {
      assert!(
        message.contains("bump.config.json"),
        "报错应含文件路径：{message}"
      );
    }
    other => panic!("应为 Parse，实际 {other:?}"),
  }
}
