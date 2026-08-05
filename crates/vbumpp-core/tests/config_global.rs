//! 全局配置层（ADR-0015）：`~/.vbumpp/config.{json,jsonc,toml}` 与项目层的
//! 四层合并（overrides > 项目 > 全局 > 内建默认）。全程注入全局目录，
//! 不碰 `VBUMPP_HOME` 环境变量（进程全局、并发竞态）。

use std::fs;

use serde_json::{json, Map, Value};
use tempfile::TempDir;
use vbumpp_core::changelog::config::resolve_changelog_config;
use vbumpp_core::config::{load_bump_config_with_home, read_document_with_home, LoadConfigError};

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn overrides(v: Value) -> Option<Map<String, Value>> {
  Some(v.as_object().unwrap().clone())
}

#[test]
fn global_config_applies_when_no_project_file() {
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(
    &home,
    "config.json",
    r#"{ "sign": true, "noVerify": true }"#,
  );
  let merged = load_bump_config_with_home(None, project.path(), Some(home.path())).unwrap();
  assert_eq!(merged["sign"], true);
  assert_eq!(merged["noVerify"], true);
  assert_eq!(merged["commit"], true, "未覆盖键回落内建默认");
}

#[test]
fn global_toml_and_jsonc_formats_parse() {
  for (name, content) in [
    ("config.toml", "sign = true\n"),
    ("config.jsonc", "// 注释\n{ \"sign\": true }"),
  ] {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    write(&home, name, content);
    let merged = load_bump_config_with_home(None, project.path(), Some(home.path())).unwrap();
    assert_eq!(merged["sign"], true, "{name} 应生效");
  }
}

#[test]
fn four_layer_precedence_chain() {
  // 内建默认 < 全局 < 项目 < overrides（同一键逐层覆盖 + 各层独有键共存）
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(&home, "config.json", r#"{ "tag": false, "sign": true }"#);
  write(
    &project,
    ".vbumpprc.json",
    r#"{ "tag": true, "push": false }"#,
  );
  let merged = load_bump_config_with_home(
    overrides(json!({ "tag": false, "noVerify": true })),
    project.path(),
    Some(home.path()),
  )
  .unwrap();
  assert_eq!(merged["tag"], false, "overrides 最高优先");
  assert_eq!(merged["push"], false, "项目层覆盖全局（全局未设 push）");
  assert_eq!(merged["sign"], true, "全局层覆盖内建默认");
  assert_eq!(merged["noVerify"], true, "overrides 独有键");
  assert_eq!(merged["commit"], true, "内建默认兜底");
}

#[test]
fn project_beats_global_same_key() {
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(&home, "config.toml", "sign = true\n");
  write(&project, ".vbumpprc.toml", "sign = false\n");
  let merged = load_bump_config_with_home(None, project.path(), Some(home.path())).unwrap();
  assert_eq!(merged["sign"], false);
}

#[test]
fn no_home_skips_global_layer() {
  let project = TempDir::new().unwrap();
  write(&project, ".vbumpprc.json", r#"{ "sign": true }"#);
  let merged = load_bump_config_with_home(None, project.path(), None).unwrap();
  assert_eq!(merged["sign"], true);
  assert_eq!(merged["noVerify"], false);
}

#[test]
fn global_ambiguity_error_lists_all() {
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(&home, "config.json", "{}");
  write(&home, "config.toml", "");
  let err = load_bump_config_with_home(None, project.path(), Some(home.path())).unwrap_err();
  match err {
    LoadConfigError::AmbiguousConfig { message } => {
      assert!(message.contains("config.json"), "{message}");
      assert!(message.contains("config.toml"), "{message}");
    }
    other => panic!("应为 AmbiguousConfig，实际 {other:?}"),
  }
}

#[test]
fn global_malformed_config_errors_with_path() {
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(&home, "config.json", "{ bad");
  let err = load_bump_config_with_home(None, project.path(), Some(home.path())).unwrap_err();
  match err {
    LoadConfigError::Parse { message } => {
      assert!(message.contains("config.json"), "{message}");
    }
    other => panic!("应为 Parse，实际 {other:?}"),
  }
}

#[test]
fn config_file_path_layer_still_stacks_over_global() {
  // configFilePath 仅替代项目层探测，全局层照常叠加（ADR-0015）
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(&home, "config.json", r#"{ "sign": true, "push": true }"#);
  write(&project, "custom.json", r#"{ "push": false }"#);
  let merged = load_bump_config_with_home(
    overrides(json!({ "configFilePath": "custom.json" })),
    project.path(),
    Some(home.path()),
  )
  .unwrap();
  assert_eq!(merged["push"], false, "custom 文件压全局");
  assert_eq!(merged["sign"], true, "全局层照常叠加");
}

#[test]
fn changelog_types_deep_merge_across_global_and_project() {
  // changelog 段 types 跨层按键深合并：项目层改标题/新增、禁用键逐层生效
  let project = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  write(
    &home,
    "config.json",
    r#"{ "changelog": { "types": { "feat": { "title": "全局特性" }, "docs": false }, "output": "GLOBAL.md" } }"#,
  );
  write(
    &project,
    ".vbumpprc.toml",
    "[changelog]\noutput = \"PROJECT.md\"\n[changelog.types.feat]\ntitle = \"项目特性\"\n[changelog.types.custom]\ntitle = \"自定义\"\n",
  );
  let document = read_document_with_home(project.path(), None, Some(home.path()))
    .unwrap()
    .unwrap();
  // 段内其余键整体替换：项目层 output 胜
  let config = resolve_changelog_config(Some(&document), None).unwrap();
  assert_eq!(config.output, "PROJECT.md");
  let titles: Vec<(&str, &str)> = config
    .types
    .iter()
    .map(|(n, e)| (n.as_str(), e.title.as_str()))
    .collect();
  assert!(
    titles.contains(&("feat", "项目特性")),
    "项目层标题覆盖全局：{titles:?}"
  );
  assert!(
    !titles.iter().any(|(n, _)| *n == "docs"),
    "全局层 false 禁用键跨层生效：{titles:?}"
  );
  assert!(
    titles.contains(&("custom", "自定义")),
    "项目层新增键进入默认表：{titles:?}"
  );
  assert!(
    titles.contains(&("fix", "🩹 Fixes")),
    "内建默认表仍在：{titles:?}"
  );
}
