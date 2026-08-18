//! napi 类型化边界与文件层形状的 round-trip 一致性：
//! 两视图键集/形状等价、serde 往返无损、`changelog.types` 声明序不被重排。

use bumpp_core_napi::config::BumpConfig;
use serde_json::{from_value, json, to_value, Map, Value};
use vbumpp_core::config::{shape_of, SUPPORTED_TOP_LEVEL_KEYS};

/// 全键覆盖 fixture：顶层 19 键 + `$schema` + overrides 专用 `configFilePath`，
/// changelog / gitlab / scripts 段全展开；`types` 刻意逆字母序声明，
/// 供声明序断言使用。键按边界结构体声明序排列（round-trip 逐字节对比锚点）
fn full_fixture() -> Value {
  json!({
    "$schema": "./vbumpprc.schema.json",
    "all": true,
    "changelog": {
      "output": "HISTORY.md",
      "types": {
        "zeta": { "title": "Zeta Group" },
        "alpha": false,
        "mid": {},
        "beta": { "title": "Beta Group", "excludeScopes": ["agent", "deps"] }
      },
      "repo": { "provider": "github", "repo": "vill-v-kit/bumpp" },
      "scopeMap": { "core": "engine", "cli": "command line" },
      "noAuthors": true,
      "hideAuthorEmail": false,
      "excludeAuthors": ["bot[0]", "bot[1]"],
      "templates": { "tagBody": "release {{newVersion}}" },
      "commitMessage": "docs: refresh {{output}}"
    },
    "commit": "chore: release v{{newVersion}}",
    "confirm": false,
    "currentVersion": "1.2.3",
    "execute": "echo done",
    "files": ["package.json", "src/version.ts"],
    "gitlab": { "host": "gitlab.example.com" },
    "ignoreScripts": true,
    "install": true,
    "noGitCheck": true,
    "noVerify": true,
    "preid": "beta",
    "push": false,
    "recursive": false,
    "release": "minor",
    "scripts": {
      "preversion": "echo pre",
      "version": "echo ver",
      "postversion": "echo post"
    },
    "sign": true,
    "tag": false,
    "configFilePath": "./custom.vbumpprc.json"
  })
}

fn object_of(value: &Value) -> &Map<String, Value> {
  value.as_object().expect("fixture 为对象")
}

#[test]
fn serde_round_trip_is_lossless() {
  let fixture = full_fixture();
  let config: BumpConfig = from_value(fixture.clone()).expect("fixture 全键合法，反序列化应成功");
  let back = to_value(&config).expect("序列化不失败");
  // 字符串对比（序列化按键插入序输出）：连顶层键序一并钉死，真「逐字节」
  assert_eq!(
    back.to_string(),
    fixture.to_string(),
    "经 napi 边界结构体的 serde 往返必须逐字节无损（含键序）"
  );
}

#[test]
fn shape_view_matches_file_layer() {
  let fixture = full_fixture();
  let map = object_of(&fixture);
  let from_file_layer = shape_of(map).expect("fixture 是文件层合法形状");
  let from_napi: BumpConfig = from_value(fixture).expect("napi 视图反序列化");
  let via_napi = shape_of(&from_napi.into_map()).expect("napi 产物过形状解析");
  assert_eq!(
    via_napi, from_file_layer,
    "文件层结构与 napi 结构对同一份配置必须解析出同一形状"
  );
}

#[test]
fn key_set_mirrors_file_layer_plus_config_file_path() {
  let config: BumpConfig = from_value(full_fixture()).expect("fixture 反序列化");
  let map = config.into_map();
  let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
  keys.sort_unstable();
  let mut expected: Vec<&str> = SUPPORTED_TOP_LEVEL_KEYS.to_vec();
  expected.push("configFilePath");
  expected.sort_unstable();
  assert_eq!(
    keys, expected,
    "napi 边界键集 = 文件层白名单键集 + overrides 专用 configFilePath"
  );
}

#[test]
fn changelog_types_keep_declaration_order() {
  let config: BumpConfig = from_value(full_fixture()).expect("fixture 反序列化");
  let map = config.into_map();
  let types = map
    .get("changelog")
    .and_then(Value::as_object)
    .and_then(|section| section.get("types"))
    .and_then(Value::as_object)
    .expect("changelog.types 段在场");
  let order: Vec<&str> = types.keys().map(String::as_str).collect();
  assert_eq!(
    order,
    vec!["zeta", "alpha", "mid", "beta"],
    "types 声明序（逆字母序）经 napi 边界不得被键序重排"
  );
}

#[test]
fn type_mismatch_is_an_error_not_a_silent_default() {
  // 静默回落通路的反面钉死：类型不符即错（运行期同理由 napi 边界报错，
  // 报错带字段名；serde 侧不带键路径前缀，断言形态文案）
  let err = from_value::<BumpConfig>(json!({ "commit": 123 })).expect_err("commit 给数字必须报错");
  assert!(
    err.to_string().contains("must be a boolean or a string"),
    "实际：{err}"
  );
  let err = from_value::<BumpConfig>(json!({ "files": "package.json" }))
    .expect_err("files 给字符串必须报错");
  assert!(err.to_string().contains("invalid type"), "实际：{err}");
  let err = from_value::<BumpConfig>(json!({
    "changelog": { "types": { "feat": 123 } }
  }))
  .expect_err("types 分组值给数字必须报错");
  assert!(
    err.to_string().contains("types.feat"),
    "报错文本带键路径，实际：{err}"
  );
  let err = from_value::<BumpConfig>(json!({
    "changelog": { "types": { "feat": { "excludeScopes": "agent" } } }
  }))
  .expect_err("excludeScopes 给字符串必须报错");
  assert!(
    err.to_string().contains("types.feat.excludeScopes"),
    "报错文本带键路径，实际：{err}"
  );
  let err = from_value::<BumpConfig>(json!({
    "changelog": { "types": { "feat": { "excludeScopes": [1] } } }
  }))
  .expect_err("excludeScopes 元素给数字必须报错");
  assert!(
    err.to_string().contains("types.feat.excludeScopes"),
    "报错文本带键路径，实际：{err}"
  );
  let err = from_value::<BumpConfig>(json!({
    "changelog": { "types": { "feat": { "excludeScopes": [""] } } }
  }))
  .expect_err("excludeScopes 元素给空串必须报错（与文件层 shape 一致）");
  assert!(
    err.to_string().contains("types.feat.excludeScopes"),
    "报错文本带键路径，实际：{err}"
  );
}

#[test]
fn absent_and_null_mean_undeclared() {
  let config: BumpConfig = from_value(json!({
    "commit": null,
    "changelog": { "types": { "feat": null }, "repo": null }
  }))
  .expect("null 是未声明语义，反序列化不得报错");
  assert!(config.commit.is_none(), "null 即未声明（None）");
  let map = config.into_map();
  assert!(
    !map.contains_key("commit"),
    "null / 缺省字段不得进合并层载体"
  );
  let types = map
    .get("changelog")
    .and_then(Value::as_object)
    .and_then(|section| section.get("types"))
    .and_then(Value::as_object)
    .expect("types 段在场");
  assert_eq!(
    types.get("feat"),
    Some(&Value::Null),
    "types 内 null 值保位（消费侧按跳过语义处理）"
  );
}
