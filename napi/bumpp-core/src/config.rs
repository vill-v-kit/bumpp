//! napi 类型化边界（ADR-0037 overrides 类型化边界）：`bumpVersion` 入参形状
//! 以 `#[napi(object)]` 结构体表达，与文件层形状结构体
//! `vbumpp_core::config::shape::BumpConfig` 成对——孤儿规则使 napi 边界 trait
//! 无法落在文件层类型上，联合字段在此以 `Either` 表达；TS 类型由 napi-derive
//! 机械生成。类型不符在边界即 napi 运行期错误（静默回落通路消除）；未知键
//! 静默丢弃是 ADR-0037 接受边界（键名把关交给 TS 编译期 excess property check）。
//!
//! 合并载体不变（ADR-0037）：结构体经 serde 转 `serde_json::Map` 进四层合并——
//! 结构体是校验与类型的载体，Map 是合并的载体。与文件层形状的差异仅
//! `configFilePath`：overrides 专用机制键，文件层白名单不收。
//! `changelog.types` 经 IndexMap 保 JS 对象声明序——serde_json 开了
//! preserve_order（napi serde-json 特性带入），声明序即 markdown 分组序，
//! 不再经无序 Map 重排。

use indexmap::IndexMap;
use napi::bindgen_prelude::Either;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Map, Value};

/// napi 边界的配置形状（与文件层 `BumpConfig` 逐键成对，另多 overrides 专用
/// 机制键 `configFilePath`）。字段全 Option——缺失 / null 即「未声明」，
/// 回落内建默认是合并层语义；serde 往返（round-trip 测试钉死两视图一致）
/// 供边界产物转 Map 与测试构造
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpConfig {
  #[napi(js_name = "$schema")]
  #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub all: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub changelog: Option<ChangelogSection>,
  #[serde(
    default,
    with = "bool_or_string",
    skip_serializing_if = "Option::is_none"
  )]
  pub commit: Option<Either<bool, String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub confirm: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub current_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub execute: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub files: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub gitlab: Option<GitlabSection>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ignore_scripts: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub install: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub no_git_check: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub no_verify: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preid: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub push: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub recursive: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub release: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scripts: Option<ScriptsSection>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sign: Option<bool>,
  #[serde(
    default,
    with = "bool_or_string",
    skip_serializing_if = "Option::is_none"
  )]
  pub tag: Option<Either<bool, String>>,
  /// overrides 专用机制键（指定配置文件路径）；文件层白名单不收
  #[serde(skip_serializing_if = "Option::is_none")]
  pub config_file_path: Option<String>,
}

impl BumpConfig {
  /// 转合并层载体：serde 序列化即得配置形 `Map`（None 字段不出键，对齐
  /// JS undefined 语义；键序为声明序，合并语义不依赖键序）
  pub fn into_map(self) -> Map<String, Value> {
    match to_value(self) {
      Ok(Value::Object(map)) => map,
      Ok(_) => unreachable!("BumpConfig 序列化产物必为对象"),
      Err(e) => unreachable!("BumpConfig 序列化不失败：{e}"),
    }
  }
}

/// `scripts` 段：三个时序槽位各自的 shell 命令串
#[napi(object)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScriptsSection {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preversion: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub postversion: Option<String>,
}

/// `gitlab` 段：自建实例 host（键名严格集由消费侧 pre-pass 把关）
#[napi(object)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GitlabSection {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub host: Option<String>,
}

/// `changelog.types` 单值形态：null 跳过、boolean 启用/禁用该组、
/// `{ title?: string, excludeScopes?: string[] }` 覆盖组标题与 scope 级
/// 排除（别名进 d.ts，schema 名对齐文件层 `TypesValue` 的 JSON Schema 名）
#[napi]
pub type ChangelogTypeValue = Option<Either<bool, ChangelogTypeEntry>>;

/// `changelog.types` 分组表：IndexMap 保 JS 对象声明序
#[napi]
pub type ChangelogTypes = IndexMap<String, ChangelogTypeValue>;

/// `changelog` 段（键集对齐文件层 `ChangelogSection` 与消费侧
/// `resolve_changelog_config`）
#[napi(object)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogSection {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output: Option<String>,
  /// type 分组表：值形态见 `ChangelogTypeValue`；IndexMap 保声明序
  /// （声明序即 markdown 分组序）
  #[serde(default, with = "types_map", skip_serializing_if = "Option::is_none")]
  pub types: Option<ChangelogTypes>,
  #[serde(default, with = "repo_union", skip_serializing_if = "Option::is_none")]
  pub repo: Option<Either<String, RepoConfig>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope_map: Option<IndexMap<String, String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub no_authors: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hide_author_email: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclude_authors: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub templates: Option<TemplatesSection>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub commit_message: Option<String>,
}

/// `changelog.templates` 段：形状上仅 `tagBody`
#[napi(object)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatesSection {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tag_body: Option<String>,
}

/// `changelog.types` 单组条目：title / excludeScopes 均缺省即 no-op
/// （按键深合并语义在消费侧）
#[napi(object)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChangelogTypeEntry {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(rename = "excludeScopes", skip_serializing_if = "Option::is_none")]
  pub exclude_scopes: Option<Vec<String>>,
}

/// `changelog.repo` 的对象形态（provider / domain / repo 三可选键）
#[napi(object)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RepoConfig {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub provider: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub domain: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub repo: Option<String>,
}

// ---------------------------------------------------------------------------
// serde 桥：联合字段（napi 侧 Either）无现成 serde 实现，逐形态手写
// ---------------------------------------------------------------------------

/// `boolean | string` 联合（commit / tag）的 serde 桥
mod bool_or_string {
  use serde::de::Error as _;
  use serde::{Deserialize, Deserializer, Serializer};
  use serde_json::Value;

  use super::{Either, Serialize};

  pub fn serialize<S: Serializer>(
    value: &Option<Either<bool, String>>,
    serializer: S,
  ) -> Result<S::Ok, S::Error> {
    match value {
      None => serializer.serialize_none(),
      Some(Either::A(b)) => b.serialize(serializer),
      Some(Either::B(s)) => s.serialize(serializer),
    }
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
  ) -> Result<Option<Either<bool, String>>, D::Error> {
    match Value::deserialize(deserializer)? {
      Value::Null => Ok(None),
      Value::Bool(b) => Ok(Some(Either::A(b))),
      Value::String(s) => Ok(Some(Either::B(s))),
      _ => Err(D::Error::custom("must be a boolean or a string")),
    }
  }
}

/// `changelog.types` 表的 serde 桥：null 值保位（消费侧跳过语义），IndexMap 保序
mod types_map {
  use serde::de::Error as _;
  use serde::{Deserialize, Deserializer, Serializer};
  use serde_json::{to_value, Map, Value};

  use super::{ChangelogTypeEntry, ChangelogTypes, Either, IndexMap, Serialize};

  pub fn serialize<S: Serializer>(
    value: &Option<ChangelogTypes>,
    serializer: S,
  ) -> Result<S::Ok, S::Error> {
    let Some(map) = value else {
      return serializer.serialize_none();
    };
    let obj: Map<String, Value> = map
      .iter()
      .map(|(name, entry)| {
        let value = match entry {
          None => Value::Null,
          Some(Either::A(b)) => Value::Bool(*b),
          Some(Either::B(entry)) => match to_value(entry) {
            Ok(Value::Object(obj)) => Value::Object(obj),
            _ => unreachable!("ChangelogTypeEntry 序列化产物必为对象"),
          },
        };
        (name.clone(), value)
      })
      .collect();
    obj.serialize(serializer)
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
  ) -> Result<Option<ChangelogTypes>, D::Error> {
    match Value::deserialize(deserializer)? {
      Value::Null => Ok(None),
      Value::Object(obj) => {
        let mut map = IndexMap::new();
        for (name, value) in obj {
          let entry = match value {
            Value::Null => None,
            Value::Bool(b) => Some(Either::A(b)),
            Value::Object(entry) => {
              let title = match entry.get("title") {
                None | Some(Value::Null) => None,
                Some(Value::String(s)) => Some(s.clone()),
                Some(_) => {
                  return Err(D::Error::custom(format!(
                    "types.{name}.title must be a string"
                  )))
                }
              };
              let exclude_scopes = match entry.get("excludeScopes") {
                None | Some(Value::Null) => None,
                Some(Value::Array(items)) => {
                  let mut scopes = Vec::with_capacity(items.len());
                  for item in items {
                    match item {
                      Value::String(s) if !s.is_empty() => scopes.push(s.clone()),
                      _ => {
                        return Err(D::Error::custom(format!(
                          "types.{name}.excludeScopes array items must be non-empty strings"
                        )))
                      }
                    }
                  }
                  Some(scopes)
                }
                Some(_) => {
                  return Err(D::Error::custom(format!(
                    "types.{name}.excludeScopes must be an array of non-empty strings"
                  )))
                }
              };
              Some(Either::B(ChangelogTypeEntry {
                title,
                exclude_scopes,
              }))
            }
            _ => {
              return Err(D::Error::custom(format!(
                "types.{name} must be false or an object {{ \"title\": string, \
                 \"excludeScopes\": string[] }}"
              )))
            }
          };
          map.insert(name, entry);
        }
        Ok(Some(map))
      }
      _ => Err(D::Error::custom("\"types\" must be an object")),
    }
  }
}

/// `string | RepoConfig` 联合（changelog.repo）的 serde 桥
mod repo_union {
  use serde::de::Error as _;
  use serde::{Deserialize, Deserializer, Serializer};
  use serde_json::Value;

  use super::{Either, RepoConfig, Serialize};

  pub fn serialize<S: Serializer>(
    value: &Option<Either<String, RepoConfig>>,
    serializer: S,
  ) -> Result<S::Ok, S::Error> {
    match value {
      None => serializer.serialize_none(),
      Some(Either::A(s)) => s.serialize(serializer),
      Some(Either::B(repo)) => repo.serialize(serializer),
    }
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
  ) -> Result<Option<Either<String, RepoConfig>>, D::Error> {
    match Value::deserialize(deserializer)? {
      Value::Null => Ok(None),
      Value::String(s) => Ok(Some(Either::A(s))),
      Value::Object(obj) => {
        let str_of = |key: &str| -> Result<Option<String>, D::Error> {
          match obj.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(D::Error::custom(format!("repo.{key} must be a string"))),
          }
        };
        Ok(Some(Either::B(RepoConfig {
          provider: str_of("provider")?,
          domain: str_of("domain")?,
          repo: str_of("repo")?,
        })))
      }
      _ => Err(D::Error::custom("must be a string or an object")),
    }
  }
}
