//! 配置形状（ADR-0037）：顶层键白名单 const 与 `BumpConfig` 结构体键集的
//! 一致性钉死——两者同为文件层校验载体（白名单报未知键、结构体报类型），
//! 漂移即配置静默失效或误报。

use std::collections::BTreeSet;

use vbumpp_core::config::{BumpConfig, SUPPORTED_TOP_LEVEL_KEYS};

#[test]
fn whitelist_matches_shape_key_set() {
  // 结构体键集自 JSON Schema 的 properties 机械提取——与 COL-102 导出同源
  let schema = serde_json::to_value(schemars::schema_for!(BumpConfig)).unwrap();
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
  // 字符串数组，$schema 为字符串——COL-102 导出前的形状基线。字段全
  // Option，schemars 1.x 对 Option<T> 输出 nullable 数组形态
  let schema = serde_json::to_value(schemars::schema_for!(BumpConfig)).unwrap();
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
  let schema = serde_json::to_value(schemars::schema_for!(BumpConfig)).unwrap();

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
