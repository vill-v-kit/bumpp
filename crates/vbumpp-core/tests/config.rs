//! loadBumpConfig 合并矩阵——对齐上游 antfu/bumpp v11 的浅展开语义：
//! `bumpConfigDefaults` ← 配置文件 ← overrides（undefined/null 剥离）。
//! 多格式与并存探测见本文件后段；全局层合并见 config_global.rs（ADR-0015）。

mod common;

use std::fs;

use serde_json::{json, Map, Value};
use tempfile::TempDir;
use vbumpp_core::config::{bump_config_defaults, load_bump_config, LoadConfigError};

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn overrides(v: Value) -> Option<Map<String, Value>> {
  Some(v.as_object().unwrap().clone())
}

fn load(dir: &TempDir, o: Option<Map<String, Value>>) -> Map<String, Value> {
  common::isolate_global_home();
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
  common::isolate_global_home();
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

// ---------------------------------------------------------------------------
// 顶层键名白名单（COL-60）：configuration.mdx 与 migration-v6.mdx 声称
// 「写错键名会直接报错（并告诉你是哪个键）」——文件层严格 schema，防配置
// 静默失效；release 是文档在册键，须过白名单并出现在 merged 里
// ---------------------------------------------------------------------------

#[test]
fn release_key_in_file_is_carried() {
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.toml", "release = \"minor\"\n");
  let merged = load(&dir, None);
  assert_eq!(merged["release"], json!("minor"));
}

#[test]
fn unknown_top_level_key_errors_naming_the_key() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.toml", "bogus_key = 1\n");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(message.contains("bogus_key"), "应指出是哪个键：{message}");
      assert!(
        message.contains(".vbumpprc.toml"),
        "应含文件路径：{message}"
      );
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn unknown_key_error_lists_supported_keys() {
  // 典型场景：release 拼成 releaze——报错列出合法键（含 release）帮人自查
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", r#"{ "releaze": "patch" }"#);
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(message.contains("releaze"), "应指出是哪个键：{message}");
      assert!(message.contains("release"), "应列出支持的键：{message}");
      assert!(message.contains("commit"), "应列出支持的键：{message}");
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn multiple_unknown_keys_are_all_reported() {
  // migration-v6.mdx：搬家时正好清理掉旧配置里无效的键——一次性全部列出
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.toml",
    "bogus_one = 1\nbogus_two = 2\ncommit = false\n",
  );
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(message.contains("bogus_one"), "非法键应全部列出：{message}");
      assert!(message.contains("bogus_two"), "非法键应全部列出：{message}");
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn all_documented_keys_are_accepted() {
  // 白名单钉住文档全集（configuration.mdx 顶层配置项表 + 机制键 noGitCheck）
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.json",
    r#"{
      "files": ["a.json"],
      "commit": false,
      "tag": true,
      "push": false,
      "sign": true,
      "all": true,
      "noVerify": true,
      "recursive": false,
      "install": true,
      "ignoreScripts": true,
      "execute": "echo hi",
      "release": "patch",
      "preid": "rc",
      "currentVersion": "1.0.0",
      "confirm": false,
      "noGitCheck": false,
      "scripts": { "preversion": "echo pre" },
      "changelog": { "output": "HISTORY.md" },
      "gitlab": { "host": "https://gitlab.example.com" }
    }"#,
  );
  let merged = load(&dir, None);
  assert_eq!(merged["release"], json!("patch"));
  assert_eq!(merged["preid"], json!("rc"));
}

#[test]
fn unknown_keys_in_overrides_are_not_validated() {
  // 上游 parity：严格键名校验只针对配置文件；编程式 overrides 无 schema
  let dir = TempDir::new().unwrap();
  let merged = load(&dir, overrides(json!({ "anything_goes": 1 })));
  assert_eq!(merged["anything_goes"], json!(1));
}

#[test]
fn config_file_path_to_ts_errors() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, "custom.ts", "export default {}");
  let err = load_bump_config(
    overrides(json!({ "configFilePath": "custom.ts" })),
    dir.path(),
  )
  .unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(
        message.contains(".json / .jsonc / .toml"),
        "报错应列出支持格式：{message}"
      );
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn malformed_json_reports_path() {
  common::isolate_global_home();
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

// ---------------------------------------------------------------------------
// 多格式（ADR-0015）：JSONC（.json/.jsonc 同 parser）+ TOML；同级并存报错
// ---------------------------------------------------------------------------

#[test]
fn json_config_accepts_comments_and_trailing_commas() {
  // JSONC：.vbumpprc.json 即享注释与尾逗号（tsconfig 先例）
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.json",
    "{\n  // 团队约定：不推 tag\n  \"tag\": false,\n  \"preid\": \"rc\",\n}\n",
  );
  let merged = load(&dir, None);
  assert_eq!(merged["tag"], false);
  assert_eq!(merged["preid"], "rc");
}

#[test]
fn jsonc_file_is_detected_as_json_alias() {
  // .jsonc 别名：照顾编辑器对 .json 内注释报错的团队场景（ADR-0015）
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.jsonc", "// 注释\n{ \"push\": false }");
  let merged = load(&dir, None);
  assert_eq!(merged["push"], false);
}

#[test]
fn toml_file_is_detected_and_parsed() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.toml",
    "tag = false\npreid = \"beta\"\nfiles = [\"a.json\", \"b.json\"]\n\
     \n[scripts]\npreversion = \"echo pre\"\n",
  );
  let merged = load(&dir, None);
  assert_eq!(merged["tag"], false);
  assert_eq!(merged["preid"], "beta");
  assert_eq!(merged["files"], json!(["a.json", "b.json"]));
  assert_eq!(merged["scripts"], json!({ "preversion": "echo pre" }));
}

#[test]
fn toml_recursive_expands_manifest_globs() {
  // recursive 语义跨格式一致：merged 为真即展开插件链模式表
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.toml", "recursive = true\n");
  let merged = load(&dir, None);
  assert_eq!(merged["files"], json!(EXPECTED_MANIFEST_GLOBS));
  assert_eq!(merged["recursive"], false);
}

#[test]
fn malformed_toml_reports_path() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.toml", "tag = \n");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::Parse { message } => {
      assert!(
        message.contains(".vbumpprc.toml"),
        "报错应含文件路径：{message}"
      );
    }
    other => panic!("应为 Parse，实际 {other:?}"),
  }
}

#[test]
fn toml_datetime_is_rejected() {
  // TOML datetime 在 JSON 值域无表达——遇到即报错（ADR-0015）
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.toml", "when = 2026-08-03T00:00:00Z\n");
  assert!(load_bump_config(None, dir.path()).is_err());
}

#[test]
fn multiple_project_configs_error_listing_all() {
  // 同级并存 = 迁移事故：报错并全部列出（ADR-0015）
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", r#"{ "tag": false }"#);
  write(&dir, ".vbumpprc.toml", "tag = false\n");
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::AmbiguousConfig { message } => {
      assert!(message.contains(".vbumpprc.json"), "{message}");
      assert!(message.contains(".vbumpprc.toml"), "{message}");
    }
    other => panic!("应为 AmbiguousConfig，实际 {other:?}"),
  }
}

#[test]
fn json_and_jsonc_alias_pair_also_errors() {
  // 别名同属一级：.json + .jsonc 并存同样报错
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, ".vbumpprc.json", "{}");
  write(&dir, ".vbumpprc.jsonc", "{}");
  assert!(matches!(
    load_bump_config(None, dir.path()).unwrap_err(),
    LoadConfigError::AmbiguousConfig { .. }
  ));
}

#[test]
fn config_file_path_loads_toml() {
  let dir = TempDir::new().unwrap();
  write(&dir, "custom.toml", "push = false\n");
  let merged = load(&dir, overrides(json!({ "configFilePath": "custom.toml" })));
  assert_eq!(merged["push"], false);
}

#[test]
fn config_file_path_loads_jsonc() {
  let dir = TempDir::new().unwrap();
  write(&dir, "custom.jsonc", "// c\n{ \"push\": false }");
  let merged = load(&dir, overrides(json!({ "configFilePath": "custom.jsonc" })));
  assert_eq!(merged["push"], false);
}

#[test]
fn config_file_path_to_yaml_errors_with_supported_formats() {
  // YAML 不支持（ADR-0015）：报错列出支持格式
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(&dir, "custom.yaml", "push: false\n");
  let err = load_bump_config(
    overrides(json!({ "configFilePath": "custom.yaml" })),
    dir.path(),
  )
  .unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(message.contains(".json / .jsonc / .toml"), "{message}");
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn jsonc_rejects_json5_loose_syntax() {
  // JSONC 仅注释与尾逗号（ADR-0015）：jsonc-parser 默认开启的 JSON5 宽松项全关
  common::isolate_global_home();
  for (name, content) in [
    ("unquoted-keys", "{ tag: false }"),
    ("single-quotes", "{ 'tag': false }"),
    ("missing-comma", "{ \"tag\": false \"push\": false }"),
    ("hex-number", "{ \"port\": 0xFF }"),
  ] {
    let dir = TempDir::new().unwrap();
    write(&dir, ".vbumpprc.json", content);
    assert!(
      load_bump_config(None, dir.path()).is_err(),
      "{name} 应被拒绝"
    );
  }
}

#[test]
fn gitlab_section_validated_at_load_regardless_of_provider() {
  // gitlab 段严格 schema 随文件加载生效（ADR-0014）：不经 gitlab release 路径也报错
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.toml",
    "[gitlab]\nhsot = \"https://typo.example\"\n",
  );
  let err = load_bump_config(None, dir.path()).unwrap_err();
  match err {
    LoadConfigError::UnsupportedConfig { message } => {
      assert!(message.contains("hsot"), "应报出 typo 键名：{message}");
      assert!(
        message.contains(".vbumpprc.toml"),
        "应含文件路径：{message}"
      );
    }
    other => panic!("应为 UnsupportedConfig，实际 {other:?}"),
  }
}

#[test]
fn gitlab_host_loads_from_config_file() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    ".vbumpprc.json",
    r#"{ "gitlab": { "host": "https://gitlab.internal" } }"#,
  );
  let merged = load(&dir, None);
  assert_eq!(
    merged["gitlab"],
    json!({ "host": "https://gitlab.internal" }),
    "gitlab 段合法时原样进入合并配置"
  );
}
