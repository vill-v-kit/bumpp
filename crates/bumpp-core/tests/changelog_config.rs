//! changelog 配置段解析（ADR-0013）：内建默认 / types 深合并 / 严格 schema / 同源同读。

use bumpp_core::changelog::config::resolve_changelog_config;
use serde_json::{json, Map, Value};

fn map(v: Value) -> Map<String, Value> {
  v.as_object().unwrap().clone()
}

#[test]
fn defaults_when_no_document_no_overrides() {
  let config = resolve_changelog_config(None, None).unwrap();
  assert_eq!(config.output, "CHANGELOG.md");
  assert!(
    config.hide_author_email,
    "hideAuthorEmail 默认翻转（ADR-0012）"
  );
  assert!(!config.no_authors);
  assert!(config.exclude_authors.is_empty());
  assert!(config.scope_map.is_empty());
  assert_eq!(config.tag_body, "v{{newVersion}}");
  assert_eq!(config.commit_message, "chore: update {{output}}");
  assert_eq!(config.repo, None);
  // 内建 types：原 JS getDefaultsChangeLogConfig 的 10 组 + BreakingChange，声明序
  let types: Vec<(&str, &str)> = config
    .types
    .iter()
    .map(|(name, entry)| (name.as_str(), entry.title.as_str()))
    .collect();
  assert_eq!(
    types,
    [
      ("feat", "🚀 特性"),
      ("perf", "🔥 性能优化"),
      ("fix", "🩹 修复"),
      ("refactor", "💅 重构"),
      ("examples", "🏀 示例"),
      ("docs", "📖 文档"),
      ("chore", "🏡 框架"),
      ("build", "📦 打包"),
      ("test", "✅ 测试"),
      ("BreakingChange", "🚨 破坏性改动"),
      ("style", "🎨 样式"),
    ]
  );
}

#[test]
fn types_deep_merge_replaces_title_keeps_position() {
  let overrides = map(json!({ "types": { "feat": { "title": "✨ 新特性" } } }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  let first = &config.types[0];
  assert_eq!(first.0, "feat", "替换保原位（声明序不变）");
  assert_eq!(first.1.title, "✨ 新特性");
  assert_eq!(config.types.len(), 11, "其余默认组保留");
}

#[test]
fn types_false_disables_group() {
  let overrides = map(json!({ "types": { "chore": false, "BreakingChange": false } }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  assert!(!config.types.iter().any(|(n, _)| n == "chore"));
  assert!(!config.types.iter().any(|(n, _)| n == "BreakingChange"));
  assert_eq!(config.types.len(), 9);
}

#[test]
fn types_empty_object_is_deep_merge_noop() {
  // 深合并语义：空对象为空合并（不报错、不改既有条目、不新增）
  let overrides = map(json!({ "types": { "feat": {}, "newtype": {} } }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  assert_eq!(config.types.len(), 11);
  assert_eq!(config.types[0].1.title, "🚀 特性");
}

#[test]
fn types_new_key_appended_in_document_order() {
  // 反字母序键名：保序特性（preserve_order）下按文档序追加，非字母序
  let overrides = map(json!({
    "types": { "zeta": { "title": "Z 组" }, "alpha": { "title": "A 组" } }
  }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  let names: Vec<&str> = config.types.iter().map(|(n, _)| n.as_str()).collect();
  assert_eq!(names.last().unwrap(), &"alpha");
  assert_eq!(
    names[names.len() - 2],
    "zeta",
    "新键按文档序追加在默认表之后"
  );
}

#[test]
fn types_invalid_values_error_with_key_name() {
  for bad in [
    json!({ "types": { "feat": "x" } }),
    json!({ "types": { "feat": { "title": 1 } } }),
    json!({ "types": { "feat": { "title": "x", "semver": "patch" } } }),
    json!({ "types": { "feat": true } }),
  ] {
    let overrides = map(bad);
    let err = resolve_changelog_config(None, Some(&overrides)).unwrap_err();
    assert!(
      err.to_string().contains("types.feat"),
      "报错应含键路径：{err}（输入 {overrides:?}）"
    );
  }
}

#[test]
fn scalar_and_whole_replace_keys() {
  let doc = map(json!({
    "changelog": {
      "output": "docs/HISTORY.md",
      "noAuthors": true,
      "hideAuthorEmail": false,
      "commitMessage": "docs: 更新 {{output}}",
      "excludeAuthors": ["bot-a"],
      "scopeMap": { "ui": "界面" },
      "templates": { "tagBody": "release-{{newVersion}}" }
    }
  }));
  let config = resolve_changelog_config(Some(&doc), None).unwrap();
  assert_eq!(config.output, "docs/HISTORY.md");
  assert!(config.no_authors);
  assert!(!config.hide_author_email);
  assert_eq!(config.commit_message, "docs: 更新 {{output}}");
  assert_eq!(config.exclude_authors, ["bot-a"]);
  assert_eq!(config.scope_map.get("ui").unwrap(), "界面");
  assert_eq!(config.tag_body, "release-{{newVersion}}");
}

#[test]
fn overrides_beat_file_and_whole_replace_arrays() {
  let doc = map(json!({ "changelog": { "output": "A.md", "excludeAuthors": ["a", "b"] } }));
  let overrides = map(json!({ "output": "B.md", "excludeAuthors": ["c"] }));
  let config = resolve_changelog_config(Some(&doc), Some(&overrides)).unwrap();
  assert_eq!(config.output, "B.md", "overrides 最高优先");
  assert_eq!(config.exclude_authors, ["c"], "数组整体替换不拼接");
}

#[test]
fn null_values_are_treated_as_absent() {
  // JS undefined 经 napi 序列化为 null，语义对齐跳过
  let overrides = map(json!({ "output": null, "hideAuthorEmail": null }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  assert_eq!(config.output, "CHANGELOG.md");
  assert!(config.hide_author_email);
}

#[test]
fn repo_string_parses_via_repo_config() {
  let overrides = map(json!({ "repo": "git@gitlab.com:owner/repo.git" }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  let repo = config.repo.unwrap();
  assert_eq!(repo.provider.as_deref(), Some("gitlab"));
  assert_eq!(repo.domain.as_deref(), Some("gitlab.com"));
  assert_eq!(repo.repo.as_deref(), Some("owner/repo"));
}

#[test]
fn repo_object_takes_fields_verbatim() {
  let overrides = map(json!({ "repo": { "repo": "owner/repo" } }));
  let config = resolve_changelog_config(None, Some(&overrides)).unwrap();
  let repo = config.repo.unwrap();
  assert_eq!(repo.provider, None);
  assert_eq!(repo.repo.as_deref(), Some("owner/repo"));
}

#[test]
fn unknown_key_errors_with_name() {
  let overrides = map(json!({ "outpot": "X.md" }));
  let err = resolve_changelog_config(None, Some(&overrides)).unwrap_err();
  assert!(
    err.to_string().contains("\"outpot\""),
    "报键名含 typo：{err}"
  );
  // cwd 非运行时特判——按未知键报错（changelogen 的 cwd 迁移者同样得到指引）
  let overrides = map(json!({ "cwd": "/tmp" }));
  let err = resolve_changelog_config(None, Some(&overrides)).unwrap_err();
  assert!(err.to_string().contains("未支持的键"), "{err}");
}

#[test]
fn legacy_keys_error() {
  for bad in [
    json!({ "tokens": { "github": "x" } }),
    json!({ "publish": {} }),
    json!({ "templates": { "commitMessage": "x" } }),
    json!({ "templates": { "tagMessage": "x" } }),
    json!({ "templates": { "unknown": "x" } }),
  ] {
    let overrides = map(bad);
    let err = resolve_changelog_config(None, Some(&overrides)).unwrap_err();
    assert!(
      err.to_string().contains("移除") || err.to_string().contains("未支持"),
      "遗产/未知键报错：{err}"
    );
  }
}

#[test]
fn runtime_keys_error_in_file_and_overrides() {
  for key in ["from", "to", "newVersion"] {
    let doc = map(json!({ "changelog": { key: "x" } }));
    let err = resolve_changelog_config(Some(&doc), None).unwrap_err();
    assert!(
      err.to_string().contains(key),
      "文件内运行时键 {key} 报错：{err}"
    );
    let overrides = map(json!({ key: "x" }));
    let err = resolve_changelog_config(None, Some(&overrides)).unwrap_err();
    assert!(
      err.to_string().contains(key),
      "overrides 运行时键 {key} 报错：{err}"
    );
  }
}

#[test]
fn changelog_section_null_is_absent() {
  // 段级 null 与键级一致：按 JS undefined 语义跳过
  let doc = map(json!({ "changelog": null }));
  let config = resolve_changelog_config(Some(&doc), None).unwrap();
  assert_eq!(config.output, "CHANGELOG.md");
}

#[test]
fn changelog_section_must_be_object() {
  let doc = map(json!({ "changelog": "x" }));
  let err = resolve_changelog_config(Some(&doc), None).unwrap_err();
  assert!(err.to_string().contains("必须是对象"), "{err}");
}

#[test]
fn invalid_scalar_types_error() {
  for bad in [
    json!({ "output": 1 }),
    json!({ "noAuthors": "yes" }),
    json!({ "excludeAuthors": "a" }),
    json!({ "excludeAuthors": [1] }),
    json!({ "scopeMap": { "ui": 1 } }),
    json!({ "repo": 1 }),
    json!({ "repo": { "unknown": "x" } }),
  ] {
    let overrides = map(bad);
    assert!(
      resolve_changelog_config(None, Some(&overrides)).is_err(),
      "应报错：{overrides:?}"
    );
  }
}

#[test]
fn bumpp_and_changelog_share_one_document() {
  // 同源同读（ADR-0013）：load_bump_config 的浅合并结果携带原始 changelog 段，
  // 同一份文档直接喂 resolve_changelog_config——单一解析路径，无二次读文件
  let dir = tempfile::TempDir::new().unwrap();
  std::fs::write(
    dir.path().join(".vbumpprc.json"),
    r#"{ "tag": false, "changelog": { "output": "docs/HISTORY.md" } }"#,
  )
  .unwrap();
  let merged = bumpp_core::config::load_bump_config(None, dir.path()).unwrap();
  assert_eq!(merged["tag"], false, "bumpp 键顶层解析");
  assert!(merged.get("changelog").is_some(), "changelog 段随文档携带");
  let config = resolve_changelog_config(Some(&merged), None).unwrap();
  assert_eq!(config.output, "docs/HISTORY.md");
}
