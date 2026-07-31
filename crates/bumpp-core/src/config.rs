//! `loadBumpConfig`：仅支持 JSON 配置文件，语义对齐上游 antfu/bumpp v11。
//!
//! 合并顺序（浅展开，整体替换）：`bumpConfigDefaults` ← 配置文件 ← overrides。
//! 上游经 napi 传入的 JS `undefined` 会序列化为 `null`，剥离时对齐上游的
//! `v !== void 0` 过滤。

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// 配置文件探测基名（上游为 `bump.config`）
const CONFIG_BASENAME: &str = "bump.config";

/// 上游支持但我们不执行的脚本扩展名（上游探测顺序），检测到即报错
const SCRIPT_EXTENSIONS: [&str; 6] = ["ts", "mts", "cts", "js", "mjs", "cjs"];

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
  let mut merged = bump_config_defaults();

  let custom_path = overrides
    .as_ref()
    .and_then(|o| o.get("configFilePath"))
    .and_then(Value::as_str)
    .map(str::to_owned);

  match custom_path {
    // 上游：指定 configFilePath 时按给定文件精确加载
    Some(custom) => {
      let path = cwd.join(custom);
      if path.extension().is_some_and(|ext| ext != "json") {
        return Err(unsupported_config(&path));
      }
      merge(&mut merged, read_config(&path)?);
    }
    None => {
      // 存在脚本配置即报错（即使 bump.config.json 同时存在）——不静默忽略
      if let Some(script) = probe_script_config(cwd) {
        return Err(unsupported_config(&script));
      }
      let json_path = cwd.join(format!("{CONFIG_BASENAME}.json"));
      if json_path.is_file() {
        merge(&mut merged, read_config(&json_path)?);
      }
    }
  }

  if let Some(overrides) = overrides {
    merge(&mut merged, strip_nulls(overrides));
  }
  Ok(merged)
}

/// 浅展开：每个键整体替换（数组、嵌套对象均不递归合并），对齐上游 `{...a, ...b}`。
fn merge(base: &mut Map<String, Value>, overrides: Map<String, Value>) {
  for (k, v) in overrides {
    base.insert(k, v);
  }
}

/// 剥离 null 值，对齐上游 `Object.entries(overrides).filter(([, v]) => v !== void 0)`。
fn strip_nulls(map: Map<String, Value>) -> Map<String, Value> {
  map.into_iter().filter(|(_, v)| !v.is_null()).collect()
}

fn probe_script_config(cwd: &Path) -> Option<PathBuf> {
  SCRIPT_EXTENSIONS
    .iter()
    .map(|ext| cwd.join(format!("{CONFIG_BASENAME}.{ext}")))
    .find(|p| p.is_file())
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
      "仅支持 JSON 配置（bump.config.json）；检测到脚本配置 {}，本实现不执行 TS/JS 配置。\
       迁移指引：将导出的配置对象写入 bump.config.json 后删除原文件。\
       注意：customVersion 等函数选项无法以 JSON 表达，已随本重写移除。",
      path.display()
    ),
  }
}
