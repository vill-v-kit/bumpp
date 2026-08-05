//! crate 内共享的 JSONC 辅助：容错解析与对象属性访问。

use jsonc_parser::ast::{Object, ObjectProp, Value};
use jsonc_parser::{parse_to_ast, CollectOptions, ParseOptions};

/// 容错解析：解析失败返回 None（对齐上游 jsonc.parse 的容错风格——坏 JSON 按无数据处理。
/// 注意与上游的差异：上游为恢复级 partial parse，坏文件里错误点之前的键仍可能可见；
/// 此处解析失败即整体 None。坏 JSON 连 npm 自身也无法读取，该偏差只影响病理场景）
pub(crate) fn parse(text: &str) -> Option<Value<'_>> {
  parse_to_ast(text, &CollectOptions::default(), &ParseOptions::default())
    .ok()
    .and_then(|ast| ast.value)
}

pub(crate) fn get_prop<'a>(obj: &'a Object, name: &str) -> Option<&'a ObjectProp<'a>> {
  obj.properties.iter().find(|p| p.name.as_str() == name)
}

/// 上游 `isManifest`：name / version / description 均为可选字符串（缺省、null、字符串）
pub(crate) fn is_manifest(root: &Object) -> bool {
  ["name", "version", "description"]
    .iter()
    .all(|key| match get_prop(root, key) {
      None => true,
      Some(prop) => matches!(&prop.value, Value::NullKeyword(_) | Value::StringLit(_)),
    })
}
