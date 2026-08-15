//! 配置形状（ADR-0037）：顶层键白名单 const 与 `BumpConfig` 结构体键集的
//! 一致性钉死——两者同为文件层校验载体（白名单报未知键、结构体报类型），
//! 漂移即配置静默失效或误报。schema 断言一律骑 `config_schema()` 导出物
//! （COL-102：`vbumpp schema` 子命令与仓库产物再生消费同一份）。

use std::collections::BTreeSet;

use serde_json::json;
use vbumpp_core::config::{config_schema, SUPPORTED_TOP_LEVEL_KEYS};

#[test]
fn whitelist_matches_shape_key_set() {
  // 结构体键集自导出 schema 的 properties 机械提取
  let schema = config_schema();
  let properties = schema["properties"]
    .as_object()
    .expect("BumpConfig 的 schema 应为 object 形态");
  let shape_keys: BTreeSet<&String> = properties.keys().collect();
  let whitelist: BTreeSet<&&str> = SUPPORTED_TOP_LEVEL_KEYS.iter().collect();

  assert_eq!(
    shape_keys.len(),
    whitelist.len(),
    "键数一致（白名单 {whitelist:?} / 结构体 {shape_keys:?}）"
  );
  for key in &whitelist {
    assert!(
      shape_keys.iter().any(|k| k.as_str() == **key),
      "白名单键 {key} 缺失于结构体"
    );
  }
  // 白名单无重复（同键两次进 const 会让报错清单重复列键）
  assert_eq!(
    whitelist.len(),
    SUPPORTED_TOP_LEVEL_KEYS.len(),
    "白名单不应有重复键"
  );
}

#[test]
fn schema_declares_expected_property_shapes() {
  // 机械导出的抽样形态钉死：commit / tag 为 bool|string 联合，files 为
  // 字符串数组，$schema 为字符串——形状基线。字段全 Option，schemars 1.x
  // 对 Option<T> 输出 nullable 数组形态
  let schema = config_schema();
  let defs = &schema["$defs"];
  let properties = schema["properties"].as_object().unwrap();

  assert_eq!(
    properties["$schema"]["type"],
    serde_json::json!(["string", "null"])
  );
  assert_eq!(
    properties["files"]["type"],
    serde_json::json!(["array", "null"])
  );
  assert_eq!(properties["files"]["items"]["type"], "string");

  let bool_or_string = &defs["BoolOrString"]["oneOf"];
  let kinds: Vec<&str> = bool_or_string
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v["type"].as_str().unwrap())
    .collect();
  assert!(
    kinds.contains(&"boolean") && kinds.contains(&"string"),
    "{kinds:?}"
  );
}

#[test]
fn schema_descriptions_are_english_only() {
  // schema 内 description 属用户可见字符串，唯一语言英文——递归扫描
  // 全树，任何 CJK 字符即中文 doc 注释泄漏回 schema
  let schema = config_schema();

  fn scan(value: &serde_json::Value, offenders: &mut Vec<String>) {
    match value {
      serde_json::Value::Object(map) => {
        for (key, child) in map {
          if key == "description" || key == "title" {
            if let Some(text) = child.as_str() {
              if text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
                offenders.push(text.to_owned());
              }
            }
          }
          scan(child, offenders);
        }
      }
      serde_json::Value::Array(items) => {
        for item in items {
          scan(item, offenders);
        }
      }
      _ => {}
    }
  }

  let mut offenders = Vec::new();
  scan(&schema, &mut offenders);
  assert!(offenders.is_empty(), "description 含中文: {offenders:?}");
}

#[test]
fn every_field_carries_a_description() {
  // schema 面向用户（编辑器提示 / SchemaStore），一个说明都不能缺：
  // 递归展开 anyOf / oneOf / $ref，凡带 properties 的层，每个属性都
  // 必须携非空 description；$defs 各定义同理
  let schema = config_schema();
  let defs = &schema["$defs"];
  let mut missing = Vec::new();

  fn walk(
    node: &serde_json::Value,
    defs: &serde_json::Value,
    path: &str,
    missing: &mut Vec<String>,
  ) {
    if let Some(r) = node.get("$ref").and_then(serde_json::Value::as_str) {
      if let Some(name) = r.strip_prefix("#/$defs/") {
        walk(&defs[name], defs, path, missing);
      }
      return;
    }
    if let Some(properties) = node
      .get("properties")
      .and_then(serde_json::Value::as_object)
    {
      for (key, prop) in properties {
        let child = if path.is_empty() {
          key.clone()
        } else {
          format!("{path}.{key}")
        };
        let described = prop
          .get("description")
          .and_then(serde_json::Value::as_str)
          .is_some_and(|text| !text.is_empty());
        if !described {
          missing.push(child.clone());
        }
        walk(prop, defs, &child, missing);
      }
    }
    for branch in ["anyOf", "oneOf", "allOf"] {
      if let Some(items) = node.get(branch).and_then(serde_json::Value::as_array) {
        for item in items {
          walk(item, defs, path, missing);
        }
      }
    }
    // BTreeMap 字段（types / scopeMap）经 additionalProperties 引用值形态
    if let Some(extra) = node.get("additionalProperties") {
      walk(extra, defs, path, missing);
    }
  }

  walk(&schema, defs, "", &mut missing);
  for (name, entry) in defs.as_object().unwrap() {
    let described = entry
      .get("description")
      .and_then(serde_json::Value::as_str)
      .is_some_and(|text| !text.is_empty());
    if !described {
      missing.push(format!("$defs.{name}"));
    }
  }
  assert!(missing.is_empty(), "缺 description 的字段: {missing:?}");
}

#[test]
fn exported_schema_pins_unknown_key_rejection_and_sections() {
  // 编辑器报红载体：顶层 additionalProperties: false（与文件层白名单
  // pre-pass 同语义）；changelog / gitlab / scripts 三段（含 templates）
  // 经 $defs 引用全量覆盖
  let schema = config_schema();
  assert_eq!(
    schema["additionalProperties"],
    json!(false),
    "顶层未知键必须被 schema 拒绝"
  );
  let defs = schema["$defs"].as_object().expect("嵌套段经 $defs 引用");
  for name in [
    "ChangelogSection",
    "GitlabSection",
    "ScriptsSection",
    "TemplatesSection",
    "BoolOrString",
    "ChangelogTypeValue",
    "ChangelogRepo",
  ] {
    assert!(defs.contains_key(name), "$defs 缺 {name}");
  }
}

#[test]
fn exported_schema_union_shapes() {
  // 三处联合形态钉死：commit / tag 的 bool|string 在抽样用例已覆盖，
  // 此处钉 types 值的 false|对象与 repo 的 string|对象（Option 包装为
  // anyOf [ref, null]，联合本体在 $defs）
  let schema = config_schema();
  let defs = &schema["$defs"];

  let types = defs["ChangelogTypeValue"]["oneOf"].as_array().unwrap();
  assert_eq!(types[0]["const"], json!(false), "false 禁用分支：{types:?}");
  assert_eq!(types[1]["type"], "object", "对象分支：{types:?}");
  assert_eq!(types[1]["properties"]["title"]["type"], "string");

  let repo = defs["ChangelogRepo"]["oneOf"].as_array().unwrap();
  let kinds: Vec<&str> = repo.iter().map(|v| v["type"].as_str().unwrap()).collect();
  assert!(
    kinds.contains(&"string") && kinds.contains(&"object"),
    "string 短写 | 对象显式：{kinds:?}"
  );
  let object_branch = repo.iter().find(|v| v["type"] == "object").unwrap();
  for key in ["provider", "domain", "repo"] {
    assert_eq!(object_branch["properties"][key]["type"], "string", "{key}");
  }
}
