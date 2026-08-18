//! JSON Schema 机械导出：形状结构体经 schemars 生成，`vbumpp schema`
//! 子命令与仓库产物再生共用同一份。顶层
//! `additionalProperties: false` 在此补入——结构体不开 `deny_unknown_fields`
//! （未知键的定制报错 UX 由文件层 pre-pass 承载），而编辑器对未知顶层键报红
//! 需要该子句。

use schemars::schema_for;
use serde_json::{to_value, Value};

use super::shape::BumpConfig;

/// 配置形状的 JSON Schema（draft 2020-12）：顶层 19 键 + `$schema` 与
/// changelog / gitlab / scripts 段全覆盖；顶层 `additionalProperties: false`
/// （未知顶层键编辑器报红，与文件层白名单 pre-pass 同语义）；description 全
/// 英文（tests/config_shape.rs 递归钉死）
pub fn config_schema() -> Value {
  let mut schema =
    to_value(schema_for!(BumpConfig)).expect("schemars Schema always serializes to serde_json");
  if let Some(root) = schema.as_object_mut() {
    root.insert("additionalProperties".to_string(), Value::Bool(false));
  }
  schema
}
