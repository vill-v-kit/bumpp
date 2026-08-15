//! changelog 配置段解析（ADR-0013）：与 bumpp 键同源同读同一份
//! `.vbumpprc.json` 文档，解析结果不向 JS 导出（全项目单一配置解析路径）。
//!
//! 合并语义：overrides > 文件 > 内建默认；`types` 按键深合并（值为 `false`
//! 即禁用该组），其余键整体替换。严格 schema：未知键、changelogen 遗产键、
//! 运行时入参键（`from` / `to` / `newVersion` / `cwd`）一律报错并报键名。

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use serde_json::{Map, Value};

use crate::git::{get_repo_config, RepoConfig};

/// 单个 type 分组的配置（changelogen 的 `ChangelogConfigType` 收窄：仅 `title`）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelogTypeEntry {
  pub title: String,
}

/// 解析后的 changelog 配置（运行时 `from` / `to` / `newVersion` 不在此——
/// 它们是 generateChangelog 的入参，ADR-0012）
#[derive(Debug, Clone, PartialEq)]
pub struct ChangelogConfig {
  pub output: String,
  /// type 分组表，声明序即 markdown 分组序（changelogen `for (const type in config.types)`）
  pub types: Vec<(String, ChangelogTypeEntry)>,
  pub repo: Option<RepoConfig>,
  pub scope_map: HashMap<String, String>,
  pub no_authors: bool,
  pub hide_author_email: bool,
  pub exclude_authors: Vec<String>,
  /// `templates.tagBody`（`## ` 头模板，`{{newVersion}}` 占位）
  pub tag_body: String,
  /// changelog 提交信息模板（`{{output}}` 占位，ADR-0012 C3）
  pub commit_message: String,
}

#[derive(Debug)]
pub enum ChangelogConfigError {
  Schema { message: String },
}

impl fmt::Display for ChangelogConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Schema { message } => f.write_str(message),
    }
  }
}

impl Error for ChangelogConfigError {}

/// 内建默认（ADR-0013）：types 键集/声明序自原 JS `getDefaultsChangeLogConfig` 迁入，
/// title 为 changelogen 英文措辞（ADR-0017：中文标题定制移出为项目级配置）；
/// `hideAuthorEmail` 默认翻转 changelogen（ADR-0012）
fn defaults() -> ChangelogConfig {
  let types = [
    ("feat", "🚀 Enhancements"),
    ("perf", "🔥 Performance"),
    ("fix", "🩹 Fixes"),
    ("refactor", "💅 Refactors"),
    ("examples", "🏀 Examples"),
    ("docs", "📖 Documentation"),
    ("chore", "🏡 Chore"),
    ("build", "📦 Build"),
    ("test", "✅ Tests"),
    ("BreakingChange", "🚨 Breaking Changes"),
    ("style", "🎨 Styles"),
  ]
  .into_iter()
  .map(|(name, title)| {
    (
      name.to_owned(),
      ChangelogTypeEntry {
        title: title.to_owned(),
      },
    )
  })
  .collect();
  ChangelogConfig {
    output: "CHANGELOG.md".to_owned(),
    types,
    repo: None,
    scope_map: HashMap::new(),
    no_authors: false,
    hide_author_email: true,
    exclude_authors: vec![],
    tag_body: "v{{newVersion}}".to_owned(),
    commit_message: "chore: update {{output}}".to_owned(),
  }
}

/// 解析 changelog 配置：`document` 为 `.vbumpprc.json` 整文档（与 `load_bump_config`
/// 同一份，取其中 `changelog` 段）；`overrides` 为用户程序化覆盖（changelog 段形状）。
/// null 值按 JS undefined 语义跳过（napi 序列化对齐）
pub fn resolve_changelog_config(
  document: Option<&Map<String, Value>>,
  overrides: Option<&Map<String, Value>>,
) -> Result<ChangelogConfig, ChangelogConfigError> {
  let mut config = defaults();
  // 段级 null 与键级一致：按 JS undefined 语义跳过（napi 序列化对齐）
  let section = match document.and_then(|doc| doc.get("changelog")) {
    None | Some(Value::Null) => None,
    Some(Value::Object(map)) => Some(map),
    Some(_) => {
      return Err(ChangelogConfigError::Schema {
        message: "the changelog section in the config file must be an object".to_owned(),
      })
    }
  };
  for source in [section, overrides].into_iter().flatten() {
    apply_section(&mut config, source)?;
  }
  Ok(config)
}

fn apply_section(
  config: &mut ChangelogConfig,
  section: &Map<String, Value>,
) -> Result<(), ChangelogConfigError> {
  for (key, value) in section {
    if value.is_null() {
      continue;
    }
    match key.as_str() {
      "output" => {
        config.output = expect_string(key, value)?.to_owned();
      }
      "noAuthors" => {
        config.no_authors = expect_bool(key, value)?;
      }
      "hideAuthorEmail" => {
        config.hide_author_email = expect_bool(key, value)?;
      }
      "commitMessage" => {
        config.commit_message = expect_string(key, value)?.to_owned();
      }
      "excludeAuthors" => {
        config.exclude_authors = expect_string_array(key, value)?;
      }
      "scopeMap" => {
        config.scope_map = expect_string_map(key, value)?;
      }
      "repo" => {
        config.repo = Some(parse_repo(value)?);
      }
      "templates" => {
        apply_templates(config, value)?;
      }
      "types" => {
        apply_types(config, value)?;
      }
      "from" | "to" | "newVersion" => {
        return Err(schema(&format!(
          "\"{key}\" is a runtime argument and must not be written to the config file"
        )));
      }
      "tokens" | "publish" => {
        return Err(schema(&format!(
          "contains changelogen legacy key \"{key}\": removed in the Rust rewrite — delete it"
        )));
      }
      _ => {
        return Err(schema(&format!(
          "contains unsupported key \"{key}\": supported keys are output / types / repo / \
           scopeMap / noAuthors / hideAuthorEmail / excludeAuthors / templates.tagBody / \
           commitMessage"
        )));
      }
    }
  }
  Ok(())
}

fn apply_templates(
  config: &mut ChangelogConfig,
  value: &Value,
) -> Result<(), ChangelogConfigError> {
  let Value::Object(map) = value else {
    return Err(schema("\"templates\" must be an object"));
  };
  for (key, value) in map {
    if value.is_null() {
      continue;
    }
    match key.as_str() {
      "tagBody" => {
        config.tag_body = expect_string("templates.tagBody", value)?.to_owned();
      }
      "commitMessage" | "tagMessage" => {
        return Err(schema(&format!(
          "templates.{key} was removed in the rewrite (changelogen's own bump-specific \
           key; this implementation's commit message key is the changelog section's \
           top-level commitMessage)"
        )));
      }
      _ => {
        return Err(schema(&format!(
          "templates contains unsupported key \"{key}\": only tagBody is supported"
        )));
      }
    }
  }
  Ok(())
}

/// types 按键深合并：对象值替换/新增（既有键保原位，新键追加声明序）；
/// `false` 禁用该组（删除既有键）；其余值形态报错
fn apply_types(config: &mut ChangelogConfig, value: &Value) -> Result<(), ChangelogConfigError> {
  let Value::Object(map) = value else {
    return Err(schema("\"types\" must be an object"));
  };
  for (name, value) in map {
    if value.is_null() {
      continue;
    }
    match value {
      Value::Bool(false) => {
        config.types.retain(|(n, _)| n != name);
      }
      Value::Object(entry) => {
        let mut title: Option<&str> = None;
        for (key, value) in entry {
          match key.as_str() {
            "title" => title = Some(expect_string(&format!("types.{name}.title"), value)?),
            _ => {
              return Err(schema(&format!(
                "types.{name} contains unsupported key \"{key}\": only title is supported"
              )));
            }
          }
        }
        // 深合并语义：空对象是 no-op（对齐「按键深合并」，不作存在性强制）
        let Some(title) = title else {
          continue;
        };
        let entry = ChangelogTypeEntry {
          title: title.to_owned(),
        };
        match config.types.iter_mut().find(|(n, _)| n == name) {
          Some((_, slot)) => *slot = entry,
          None => config.types.push((name.clone(), entry)),
        }
      }
      _ => {
        return Err(schema(&format!(
          "types.{name} must be {{ \"title\": string }} or false"
        )));
      }
    }
  }
  Ok(())
}

/// `repo`（changelogen `string | RepoConfig` 联合形态）：string 经
/// git::get_repo_config 解析；object 直取 provider / domain / repo 三可选字段（严格键集）
fn parse_repo(value: &Value) -> Result<RepoConfig, ChangelogConfigError> {
  match value {
    Value::String(s) => Ok(get_repo_config(s)),
    Value::Object(map) => {
      let mut repo = RepoConfig {
        provider: None,
        domain: None,
        repo: None,
      };
      for (key, value) in map {
        if value.is_null() {
          continue;
        }
        let field = expect_string(&format!("repo.{key}"), value)?.to_owned();
        match key.as_str() {
          "provider" => repo.provider = Some(field),
          "domain" => repo.domain = Some(field),
          "repo" => repo.repo = Some(field),
          _ => {
            return Err(schema(&format!(
              "repo contains unsupported key \"{key}\": only provider / domain / repo \
               are supported"
            )));
          }
        }
      }
      Ok(repo)
    }
    _ => Err(schema("\"repo\" must be a string or an object")),
  }
}

fn expect_string<'a>(key: &str, value: &'a Value) -> Result<&'a str, ChangelogConfigError> {
  value
    .as_str()
    .ok_or_else(|| schema(&format!("\"{key}\" must be a string")))
}

fn expect_bool(key: &str, value: &Value) -> Result<bool, ChangelogConfigError> {
  value
    .as_bool()
    .ok_or_else(|| schema(&format!("\"{key}\" must be a boolean")))
}

fn expect_string_array(key: &str, value: &Value) -> Result<Vec<String>, ChangelogConfigError> {
  let Some(items) = value.as_array() else {
    return Err(schema(&format!("\"{key}\" must be an array")));
  };
  items
    .iter()
    .map(|item| {
      item
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| schema(&format!("\"{key}\" array items must be strings")))
    })
    .collect()
}

fn expect_string_map(
  key: &str,
  value: &Value,
) -> Result<HashMap<String, String>, ChangelogConfigError> {
  let Value::Object(map) = value else {
    return Err(schema(&format!("\"{key}\" must be an object")));
  };
  map
    .iter()
    .map(|(k, v)| {
      v.as_str()
        .map(|s| (k.clone(), s.to_owned()))
        .ok_or_else(|| schema(&format!("\"{key}.{k}\" must be a string")))
    })
    .collect()
}

fn schema(message: &str) -> ChangelogConfigError {
  ChangelogConfigError::Schema {
    message: format!("changelog config: {message}"),
  }
}
