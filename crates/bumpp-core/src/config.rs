//! `loadBumpConfig`：仅支持 JSON 配置文件（ADR-0013），语义对齐上游 antfu/bumpp v11。
//!
//! 合并顺序（浅展开，整体替换）：`bumpConfigDefaults` ← 配置文件 ← overrides。
//! 上游经 napi 传入的 JS `undefined` 会序列化为 `null`，剥离时对齐上游的
//! `v !== void 0` 过滤。
//!
//! loader 只认 `.vbumpprc.json`（或 `configFilePath` override）：旧名
//! （`bump.config.*` / `vbumpp.config.*` / `changelog.config.*`）不探测、
//! 不读取、不报错，静默失效（ADR-0013）。

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::plugins::recursive_manifest_globs;
/// 配置文件探测：唯一文件名（ADR-0013：单一文件名，无旧名探测）
const CONFIG_FILE: &str = ".vbumpprc.json";

#[derive(Debug)]
pub enum LoadConfigError {
  Io { message: String },
  Parse { message: String },
  UnsupportedConfig { message: String },
}

impl fmt::Display for LoadConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io { message } | Self::Parse { message } | Self::UnsupportedConfig { message } => {
        f.write_str(message)
      }
    }
  }
}

impl Error for LoadConfigError {}

/// 上游 `bumpConfigDefaults`，逐键对齐。
/// 注意：上游的 `configFilePath: undefined` 在 JSON 中无从表达，不放默认值。
pub fn bump_config_defaults() -> Map<String, Value> {
  let mut m = Map::new();
  m.insert("commit".into(), true.into());
  m.insert("push".into(), true.into());
  m.insert("tag".into(), true.into());
  m.insert("sign".into(), false.into());
  m.insert("install".into(), false.into());
  m.insert("recursive".into(), false.into());
  m.insert("noVerify".into(), false.into());
  m.insert("confirm".into(), true.into());
  m.insert("ignoreScripts".into(), false.into());
  m.insert("all".into(), false.into());
  m.insert("noGitCheck".into(), true.into());
  m.insert("files".into(), Value::Array(vec![]));
  m
}

/// 加载并合并配置。`overrides` 对应上游的第一个参数，`cwd` 为配置探测目录。
pub fn load_bump_config(
  overrides: Option<Map<String, Value>>,
  cwd: &Path,
) -> Result<Map<String, Value>, LoadConfigError> {
  let document = read_document(cwd, custom_config_path(overrides.as_ref()).as_deref())?;
  Ok(merge_bump_config(overrides, document))
}

/// `overrides` 中的 `configFilePath`（上游：指定时按给定文件精确加载）
pub(crate) fn custom_config_path(overrides: Option<&Map<String, Value>>) -> Option<String> {
  overrides
    .and_then(|o| o.get("configFilePath"))
    .and_then(Value::as_str)
    .map(str::to_owned)
}

/// 合并语义（单一实现，ADR-0013）：`bumpConfigDefaults` ← 文档 ← overrides，
/// 随后 recursive 展开与 files 去重。与 `read_document` 组合即 `load_bump_config`；
/// changelog 编排持同一份文档另行解析 changelog 段，零二次读取
pub(crate) fn merge_bump_config(
  overrides: Option<Map<String, Value>>,
  document: Option<Map<String, Value>>,
) -> Map<String, Value> {
  let mut merged = bump_config_defaults();

  if let Some(document) = document {
    merge(&mut merged, document);
  }

  if let Some(overrides) = overrides {
    merge(&mut merged, strip_nulls(overrides));
  }

  // recursive 展开收归加载器（ADR-0013，原 JS 后处理）：merged recursive 为真时
  // 追加插件底座链 recursive 模式表并置 false（ADR-0003 opt-in 语义不变）；
  // files 非数组属病理用法，不动其值，仅消费 recursive 标志
  if merged.get("recursive").and_then(Value::as_bool) == Some(true) {
    if let Some(files) = merged.get_mut("files").and_then(Value::as_array_mut) {
      files.extend(recursive_manifest_globs().into_iter().map(Value::String));
    }
    merged.insert("recursive".into(), false.into());
  }

  // files 去重（保序，首次出现为准）——原 JS 后处理的无条件去重随加载器一并迁移
  if let Some(files) = merged.get_mut("files").and_then(Value::as_array_mut) {
    let mut seen = std::collections::HashSet::new();
    files.retain(|f| f.as_str().is_none_or(|s| seen.insert(s.to_owned())));
  }
  merged
}

/// 浅展开：每个键整体替换（数组、嵌套对象均不递归合并），对齐上游 `{...a, ...b}`。
fn merge(base: &mut Map<String, Value>, overrides: Map<String, Value>) {
  for (k, v) in overrides {
    base.insert(k, v);
  }
}

/// 配置文件读取原语（单一解析路径，ADR-0013）：bumpp 键与 changelog 段共享同一份
/// 文档。`config_file_path` 指定时按给定文件精确加载（非 .json 扩展名报错，文件
/// 缺失报 Io——上游行为）；否则探测 `.vbumpprc.json`，不存在返回 None
pub(crate) fn read_document(
  cwd: &Path,
  config_file_path: Option<&str>,
) -> Result<Option<Map<String, Value>>, LoadConfigError> {
  match config_file_path {
    Some(custom) => {
      let path = cwd.join(custom);
      if path.extension().is_some_and(|ext| ext != "json") {
        return Err(unsupported_config(&path));
      }
      Ok(Some(read_config(&path)?))
    }
    None => {
      // 唯一探测点：`.vbumpprc.json`；旧名不探测、静默失效（ADR-0013）
      let json_path = cwd.join(CONFIG_FILE);
      if json_path.is_file() {
        Ok(Some(read_config(&json_path)?))
      } else {
        Ok(None)
      }
    }
  }
}

/// 剥离 null 值，对齐上游 `Object.entries(overrides).filter(([, v]) => v !== void 0)`。
fn strip_nulls(map: Map<String, Value>) -> Map<String, Value> {
  map.into_iter().filter(|(_, v)| !v.is_null()).collect()
}

fn read_config(path: &Path) -> Result<Map<String, Value>, LoadConfigError> {
  let content = fs::read_to_string(path).map_err(|e| LoadConfigError::Io {
    message: format!("读取配置文件 {} 失败：{e}", path.display()),
  })?;
  let value: Value = serde_json::from_str(&content).map_err(|e| LoadConfigError::Parse {
    message: format!("解析配置文件 {} 失败：{e}", path.display()),
  })?;
  match value {
    Value::Object(map) => {
      if map.contains_key("customVersion") {
        return Err(LoadConfigError::UnsupportedConfig {
          message: format!(
            "配置文件 {} 包含 customVersion 选项：JSON 配置无法承载函数，\
             该选项已随本重写移除，请删除该键。",
            path.display()
          ),
        });
      }
      Ok(map)
    }
    // 上游 `{...非对象}` 的行为（数组展开为索引键等）属于病理用法，明确拒绝
    _ => Err(LoadConfigError::Parse {
      message: format!("配置文件 {} 必须是 JSON 对象", path.display()),
    }),
  }
}

fn unsupported_config(path: &Path) -> LoadConfigError {
  LoadConfigError::UnsupportedConfig {
    message: format!(
      "仅支持 JSON 配置（.vbumpprc.json 或 configFilePath 指向的 .json 文件）；\
       指定的配置文件 {} 不是 JSON，本实现不执行 TS/JS 配置。",
      path.display()
    ),
  }
}
