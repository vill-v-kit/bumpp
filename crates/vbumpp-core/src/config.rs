//! `loadBumpConfig`：语义对齐上游 antfu/bumpp v11；配置文件为两级多格式
//! （ADR-0013）：项目级 `.vbumpprc.{json,jsonc,toml}` + 全局级
//! `~/.vbumpp/config.{json,jsonc,toml}`（`.json`/`.jsonc` 同走 JSONC 解析）。
//!
//! 文件层校验（ADR-0037）键名 + 类型双重：键名 pre-pass（`customVersion` 迁移文案、
//! 未知键全列、gitlab 严格键集）在前，形状结构体反序列化在后——类型不符报错指出键路径与期望类型，不再静默回落。
//!
//! 合并顺序（浅展开，整体替换）：`bumpConfigDefaults` ← 全局文件 ← 项目文件
//! ← overrides；changelog 段例外——`types` 按键深合并逐层生效（ADR-0013）。
//! 上游经 napi 传入的 JS `undefined` 会序列化为 `null`，剥离时对齐上游的
//! `v !== void 0` 过滤。
//!
//! 探测：同级认名集合内命中 2 个及以上即报错（多配置并存几乎一定是迁移事故）；`configFilePath`
//! override 按扩展名 `.json` / `.jsonc` / `.toml` 分派，指定时替代项目层探测，全局层照常叠加；
//! 旧名（`bump.config.*` / `vbumpp.config.*` / `changelog.config.*`）不探测、不读取，静默失效（ADR-0013）。

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::display;
use crate::home::vbumpp_home;
use crate::plugins::recursive_manifest_globs;

pub mod shape;

pub use shape::{shape_of, BoolOrString, BumpConfig};

/// 项目级探测文件名集合（ADR-0013）
const PROJECT_CONFIG_FILES: [&str; 3] = [".vbumpprc.json", ".vbumpprc.jsonc", ".vbumpprc.toml"];
/// 全局级探测文件名集合（全局配置目录 `~/.vbumpp/` 内）
const GLOBAL_CONFIG_FILES: [&str; 3] = ["config.json", "config.jsonc", "config.toml"];

/// 顶层键名白名单（COL-60）：configuration.mdx 顶层配置项表 + 机制键
/// （`noGitCheck`）+ 编辑器 schema 关联键（`$schema`，ADR-0037）。文件层严格
/// schema 防配置静默失效；overrides 层不校验（编程式 API 上游 parity）。
/// `configFilePath` 不在列：overrides 专用机制键，配置文件内自指无意义。
/// 与 `BumpConfig` 结构体键集的一致性由测试钉死（tests/config_shape.rs）
pub const SUPPORTED_TOP_LEVEL_KEYS: [&str; 20] = [
  "$schema",
  "all",
  "changelog",
  "commit",
  "confirm",
  "currentVersion",
  "execute",
  "files",
  "gitlab",
  "ignoreScripts",
  "install",
  "noGitCheck",
  "noVerify",
  "preid",
  "push",
  "recursive",
  "release",
  "scripts",
  "sign",
  "tag",
];

#[derive(Debug)]
pub enum LoadConfigError {
  Io { message: String },
  Parse { message: String },
  UnsupportedConfig { message: String },
  AmbiguousConfig { message: String },
}

impl fmt::Display for LoadConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io { message }
      | Self::Parse { message }
      | Self::UnsupportedConfig { message }
      | Self::AmbiguousConfig { message } => f.write_str(message),
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
  load_bump_config_with_home(overrides, cwd, vbumpp_home().as_deref())
}

/// `load_bump_config` 的可注入形态：全局目录显式给出（None = 跳过全局层），
/// 供测试摆脱 `VBUMPP_HOME` 环境变量（进程全局、并发竞态）
#[doc(hidden)]
pub fn load_bump_config_with_home(
  overrides: Option<Map<String, Value>>,
  cwd: &Path,
  home: Option<&Path>,
) -> Result<Map<String, Value>, LoadConfigError> {
  let document =
    read_document_with_home(cwd, custom_config_path(overrides.as_ref()).as_deref(), home)?;
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
    let mut seen = HashSet::new();
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

/// 配置文件读取原语（单一解析路径，ADR-0013）：bumpp 键与 changelog 段共享
/// 同一份文档。项目层与全局层（`~/.vbumpp/`）分层读取后合并（全局 ← 项目）。
/// `config_file_path` 指定时按给定文件精确加载（替代项目层探测；文件缺失报 Io，
/// 非支持扩展名报错——上游行为扩展）；全局层照常叠加。
pub(crate) fn read_document(
  cwd: &Path,
  config_file_path: Option<&str>,
) -> Result<Option<Map<String, Value>>, LoadConfigError> {
  read_document_with_home(cwd, config_file_path, vbumpp_home().as_deref())
}

/// `read_document` 的可注入形态：全局目录显式给出（None = 跳过全局层），
/// 供测试摆脱 `VBUMPP_HOME` 环境变量（进程全局、并发竞态）
#[doc(hidden)]
pub fn read_document_with_home(
  cwd: &Path,
  config_file_path: Option<&str>,
  home: Option<&Path>,
) -> Result<Option<Map<String, Value>>, LoadConfigError> {
  let project = match config_file_path {
    Some(custom) => Some(read_config(&cwd.join(custom), cwd)?),
    None => probe_config(cwd, &PROJECT_CONFIG_FILES, cwd)?,
  };
  let global = match home {
    Some(dir) => probe_config(dir, &GLOBAL_CONFIG_FILES, cwd)?,
    None => None,
  };
  Ok(match (global, project) {
    (None, None) => None,
    (Some(g), None) => Some(g),
    (None, Some(p)) => Some(p),
    (Some(g), Some(p)) => Some(merge_config_documents(g, p)),
  })
}

/// 单级探测：认名集合内命中 1 个即解析；2 个及以上报错并全部列出（ADR-0013）。
/// `cwd` 为错误消息的显示路径锚点（ADR-0002）：项目层 dir == cwd 打相对，
/// 全局层（home 目录）打绝对 POSIX
fn probe_config(
  dir: &Path,
  names: &[&str],
  cwd: &Path,
) -> Result<Option<Map<String, Value>>, LoadConfigError> {
  let found: Vec<PathBuf> = names
    .iter()
    .map(|n| dir.join(n))
    .filter(|p| p.is_file())
    .collect();
  match found.len() {
    0 => Ok(None),
    1 => Ok(Some(read_config(&found[0], cwd)?)),
    _ => Err(LoadConfigError::AmbiguousConfig {
      message: format!(
        "multiple config files found in {}: {} — keep only one (supported: {})",
        display::path(cwd, dir),
        found
          .iter()
          .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
          .collect::<Vec<_>>()
          .join(", "),
        names.join(" / ")
      ),
    }),
  }
}

/// 文档层合并（全局 ← 项目）：顶层键浅替换，唯一例外 `changelog` 段——
/// 其 `types` 按键深合并（其余键整体替换），逐层对齐 ADR-0013 的段合并语义
fn merge_config_documents(
  base: Map<String, Value>,
  over: Map<String, Value>,
) -> Map<String, Value> {
  let mut merged = base;
  for (key, value) in over {
    match (key.as_str(), merged.get("changelog"), &value) {
      ("changelog", Some(Value::Object(base_section)), Value::Object(over_section)) => {
        merged.insert(
          "changelog".into(),
          Value::Object(merge_changelog_sections(
            base_section.clone(),
            over_section.clone(),
          )),
        );
      }
      _ => {
        merged.insert(key, value);
      }
    }
  }
  merged
}

/// changelog 段层合并：`types` 按键覆盖/禁用（`false` 即删键——与
/// `resolve_changelog_config` 的深合并语义一致），段内其余键整体替换
fn merge_changelog_sections(
  base: Map<String, Value>,
  over: Map<String, Value>,
) -> Map<String, Value> {
  let mut merged = base;
  for (key, value) in over {
    match (key.as_str(), merged.get("types"), &value) {
      ("types", Some(Value::Object(base_types)), Value::Object(over_types)) => {
        let mut types = base_types.clone();
        for (name, entry) in over_types {
          if matches!(entry, Value::Bool(false)) {
            types.remove(name.as_str());
          } else {
            types.insert(name.clone(), entry.clone());
          }
        }
        merged.insert("types".into(), Value::Object(types));
      }
      _ => {
        merged.insert(key, value);
      }
    }
  }
  merged
}

/// 剥离 null 值，对齐上游 `Object.entries(overrides).filter(([, v]) => v !== void 0)`。
fn strip_nulls(map: Map<String, Value>) -> Map<String, Value> {
  map.into_iter().filter(|(_, v)| !v.is_null()).collect()
}

/// 支持的配置文件格式（按扩展名分派）
enum ConfigFormat {
  /// `.json` / `.jsonc` 同走 JSONC 解析（注释、尾逗号可用；`.jsonc` 为别名，ADR-0013）
  Jsonc,
  Toml,
}

impl ConfigFormat {
  fn of(path: &Path) -> Option<Self> {
    match path.extension().and_then(|ext| ext.to_str()) {
      Some("json" | "jsonc") => Some(Self::Jsonc),
      Some("toml") => Some(Self::Toml),
      _ => None,
    }
  }
}

/// 单文件读取与校验：`cwd` 为错误消息的显示路径锚点（ADR-0002）
fn read_config(path: &Path, cwd: &Path) -> Result<Map<String, Value>, LoadConfigError> {
  let content = fs::read_to_string(path).map_err(|e| LoadConfigError::Io {
    message: format!(
      "failed to read config file {}: {e}",
      display::path(cwd, path)
    ),
  })?;
  let value: Value = match ConfigFormat::of(path) {
    Some(ConfigFormat::Jsonc) => {
      jsonc_parser::parse_to_serde_value(&content, &CONFIG_JSONC_OPTIONS).map_err(|e| {
        LoadConfigError::Parse {
          message: format!(
            "failed to parse config file {}: {e}",
            display::path(cwd, path)
          ),
        }
      })?
    }
    Some(ConfigFormat::Toml) => {
      let toml_value: toml::Value =
        toml::from_str(&content).map_err(|e| LoadConfigError::Parse {
          message: format!(
            "failed to parse config file {}: {e}",
            display::path(cwd, path)
          ),
        })?;
      reject_toml_datetimes(&toml_value, path, cwd)?;
      serde_json::to_value(toml_value).map_err(|e| LoadConfigError::Parse {
        message: format!(
          "failed to parse config file {}: {e}",
          display::path(cwd, path)
        ),
      })?
    }
    None => return Err(unsupported_config(path, cwd)),
  };
  match value {
    Value::Object(map) => {
      if map.contains_key("customVersion") {
        return Err(LoadConfigError::UnsupportedConfig {
          message: format!(
            "config file {} contains the customVersion option: config files cannot carry \
             functions; this option was removed in the rewrite — delete the key.",
            display::path(cwd, path)
          ),
        });
      }
      // 顶层键名白名单（COL-60）：未知键全部列出（一次报错清完旧配置里的
      // 无效键），并附合法键全集；customVersion 已在上文以专属迁移信息拦截
      let mut unknown: Vec<&str> = map
        .keys()
        .map(String::as_str)
        .filter(|k| !SUPPORTED_TOP_LEVEL_KEYS.contains(k))
        .collect();
      if !unknown.is_empty() {
        unknown.sort_unstable();
        return Err(LoadConfigError::UnsupportedConfig {
          message: format!(
            "config file {} contains unsupported {}: {} — supported top-level keys: {}",
            display::path(cwd, path),
            if unknown.len() == 1 { "key" } else { "keys" },
            unknown
              .iter()
              .map(|k| format!("\"{k}\""))
              .collect::<Vec<_>>()
              .join(", "),
            SUPPORTED_TOP_LEVEL_KEYS.join(" / ")
          ),
        });
      }
      // gitlab 段严格 schema（ADR-0014）：随文件加载即校验，与 provider 无关
      if let Some(message) = gitlab_section_error(&map) {
        return Err(LoadConfigError::UnsupportedConfig {
          message: format!("config file {}: {message}", display::path(cwd, path)),
        });
      }
      // 类型校验（ADR-0037）：键名 pre-pass 之后经形状结构体反序列化——类型不符
      // 从静默回落默认改为报错（键路径 + 期望类型）；未知键各有定制 pre-pass 文案
      shape::shape_of(&map).map_err(|message| LoadConfigError::UnsupportedConfig {
        message: format!("config file {}: {message}", display::path(cwd, path)),
      })?;
      Ok(map)
    }
    // 上游 `{...非对象}` 的行为（数组展开为索引键等）属于病理用法，明确拒绝
    _ => Err(LoadConfigError::Parse {
      message: format!("config file {} must be an object", display::path(cwd, path)),
    }),
  }
}

/// 配置文件的 JSONC 选项：仅注释与尾逗号（ADR-0013）——关掉 jsonc-parser 默认
/// 开启的 JSON5 风格宽松项（未引号键、单引号串、缺省逗号、十六进制/一元加号数字）
const CONFIG_JSONC_OPTIONS: jsonc_parser::ParseOptions = jsonc_parser::ParseOptions {
  allow_comments: true,
  allow_loose_object_property_names: false,
  allow_trailing_commas: true,
  allow_missing_commas: false,
  allow_single_quoted_strings: false,
  allow_hexadecimal_numbers: false,
  allow_unary_plus_numbers: false,
};

/// `gitlab` 段提取（ADR-0014，严格 schema：仅 `host` 一键）：
/// 段缺失/null 为 None；段或键形态非法时报错。文件层（`read_config`）与
/// overrides 层（release `resolve_gitlab_host`）共用同一校验
pub(crate) fn gitlab_host_of(
  source: &Map<String, Value>,
) -> Result<Option<String>, LoadConfigError> {
  match source.get("gitlab") {
    None | Some(Value::Null) => Ok(None),
    Some(Value::Object(map)) => {
      let mut host = None;
      for (key, value) in map {
        if value.is_null() {
          continue;
        }
        match key.as_str() {
          "host" => {
            host = Some(value.as_str().map(str::to_owned).ok_or_else(|| {
              LoadConfigError::UnsupportedConfig {
                message: "gitlab section \"host\" must be a string".into(),
              }
            })?);
          }
          _ => {
            return Err(LoadConfigError::UnsupportedConfig {
              message: format!(
                "gitlab section contains unsupported key \"{key}\": only host is supported"
              ),
            })
          }
        }
      }
      Ok(host)
    }
    Some(_) => Err(LoadConfigError::UnsupportedConfig {
      message: "gitlab section must be an object".into(),
    }),
  }
}

/// `gitlab_host_of` 的校验形态（丢弃提取值）：错误信息拼成「…的 gitlab 段…」句式
fn gitlab_section_error(source: &Map<String, Value>) -> Option<String> {
  gitlab_host_of(source).err().map(|e| e.to_string())
}

/// TOML datetime 在 JSON 值域无表达（serde 会静默降为 ISO 字符串），出现即报错（ADR-0013）
fn reject_toml_datetimes(
  value: &toml::Value,
  path: &Path,
  cwd: &Path,
) -> Result<(), LoadConfigError> {
  match value {
    toml::Value::Datetime(dt) => Err(LoadConfigError::Parse {
      message: format!(
        "config file {} contains a TOML datetime value ({dt}): not expressible in the \
         JSON value domain — use a string instead",
        display::path(cwd, path)
      ),
    }),
    toml::Value::Array(items) => items
      .iter()
      .try_for_each(|v| reject_toml_datetimes(v, path, cwd)),
    toml::Value::Table(table) => table
      .values()
      .try_for_each(|v| reject_toml_datetimes(v, path, cwd)),
    _ => Ok(()),
  }
}

fn unsupported_config(path: &Path, cwd: &Path) -> LoadConfigError {
  LoadConfigError::UnsupportedConfig {
    message: format!(
      "supported config file formats: .json / .jsonc / .toml (.vbumpprc.* or the file \
       configFilePath points to); the given config file {} is not supported — this \
       implementation does not execute TS/JS config.",
      display::path(cwd, path)
    ),
  }
}
