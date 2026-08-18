//! 配置形状（单一事实源）：`.vbumpprc.*` 顶层键集与 changelog /
//! gitlab / scripts 段的形状以 serde `Deserialize` + schemars `JsonSchema`
//! 结构体表达。两个载体身份——文件层类型校验（`read_config` 键名 pre-pass
//! 之后）与 JSON Schema 机械导出；合并载体维持 `serde_json::Map` 不变
//! （浅替换 + `changelog.types` 按键深合并是值域操作）。
//!
//! 键名定制 UX 不由本结构体承载：顶层未知键（一次全列 + 合法键集）、
//! `customVersion` 迁移文案、gitlab 段严格键集、changelog 遗留键 / 运行时键
//! 各有 pre-pass 专属文案，serde derive 拿不到这些形态——故本结构体不开
//! `deny_unknown_fields`，嵌套段内的未知键留给各自 pre-pass 报错。

use std::borrow::Cow;
use std::collections::BTreeMap;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

/// 配置形状（顶层 19 键 + `$schema`）。字段全 Option——缺失 / null 即
/// 「未声明」，回落内建默认是合并层语义；结构体只表达形状不表达默认
#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(
  description = "Bump configuration shape (19 top-level keys plus `$schema`). Every field is optional: absence means \"not declared\" and defaults are resolved at the merge layer."
)]
pub struct BumpConfig {
  /// 编辑器 schema 关联键（JSON / TOML 均合法）；不参与任何运行语义
  #[serde(rename = "$schema")]
  #[schemars(
    description = "Editor schema association key (valid in both JSON and TOML); carries no runtime semantics."
  )]
  pub schema: Option<String>,
  /// `git commit --all`（无 pathspec 全量提交）
  #[schemars(
    description = "Stage all changes when committing (`git add -A`); by default only the updated files are committed."
  )]
  pub all: Option<bool>,
  #[schemars(description = "Changelog generation settings.")]
  pub changelog: Option<ChangelogSection>,
  #[schemars(
    description = "Whether to git commit the version bump; a string is used as the custom commit message."
  )]
  pub commit: Option<BoolOrString>,
  #[schemars(
    description = "Ask for confirmation before execution; not asked again after picking a version interactively."
  )]
  pub confirm: Option<bool>,
  #[schemars(description = "Manually specify the current version, overriding auto-detection.")]
  pub current_version: Option<String>,
  #[schemars(description = "An extra command to run after the version update.")]
  pub execute: Option<String>,
  #[schemars(
    description = "Files whose version number should be updated (glob patterns); when empty, common manifest files are auto-detected."
  )]
  pub files: Option<Vec<String>>,
  #[schemars(description = "Self-hosted GitLab release settings.")]
  pub gitlab: Option<GitlabSection>,
  #[schemars(description = "Skip all lifecycle scripts.")]
  pub ignore_scripts: Option<bool>,
  #[schemars(
    description = "Run the ecosystem install after the version update (JavaScript: package-manager install; Cargo: `cargo check --workspace`)."
  )]
  pub install: Option<bool>,
  #[schemars(
    description = "Skip the git state check before bumping; defaults to skipping (mechanism key)."
  )]
  pub no_git_check: Option<bool>,
  #[schemars(description = "Skip git hooks when committing and tagging.")]
  pub no_verify: Option<bool>,
  #[schemars(description = "Prerelease identifier (the `beta` in `1.0.0-beta.1`).")]
  pub preid: Option<String>,
  #[schemars(description = "Whether to git push after the bump.")]
  pub push: Option<bool>,
  #[schemars(description = "Recursively bump the whole tree (monorepo); equivalent to `-r`.")]
  pub recursive: Option<bool>,
  #[schemars(
    description = "Skip the interactive menu and use the given bump type (e.g. `patch`) or explicit version directly."
  )]
  pub release: Option<String>,
  #[schemars(description = "Lifecycle scripts run at each phase of the bump.")]
  pub scripts: Option<ScriptsSection>,
  #[schemars(description = "GPG-sign the commit and tag.")]
  pub sign: Option<bool>,
  #[schemars(
    description = "Whether to create a git tag; a string is used as the custom tag name."
  )]
  pub tag: Option<BoolOrString>,
}

/// 形状解析（校验 + 类型化视图二合一）：类型不符即错，报错带键路径与
/// 期望类型（serde_path_to_error 前缀，如 `files: invalid type: ...`）。
/// 未知键不在此报——顶层白名单 pre-pass 与嵌套段各自 pre-pass 承载
pub fn shape_of(map: &Map<String, Value>) -> Result<BumpConfig, String> {
  // serde_path_to_error 需要 Deserializer：经序列化回读取得（配置规模下
  // 往返成本可忽略），换取键路径进报错文本
  let text = serde_json::to_string(map).expect("serde_json::Map 序列化不失败");
  let mut de = serde_json::Deserializer::from_str(&text);
  serde_path_to_error::deserialize(&mut de).map_err(|e| e.to_string())
}

/// `boolean | string` 联合形态（commit / tag）。手写 Deserialize：报错
/// 「键路径: must be a boolean or a string」，比 serde untagged derive 的
/// 「did not match any variant」可读
#[derive(Debug, Clone, PartialEq)]
pub enum BoolOrString {
  Bool(bool),
  Str(String),
}

impl<'de> Deserialize<'de> for BoolOrString {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    match Value::deserialize(deserializer)? {
      Value::Bool(b) => Ok(Self::Bool(b)),
      Value::String(s) => Ok(Self::Str(s)),
      _ => Err(D::Error::custom("must be a boolean or a string")),
    }
  }
}

impl JsonSchema for BoolOrString {
  fn schema_name() -> Cow<'static, str> {
    "BoolOrString".into()
  }

  fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({
      "description": "A boolean, or a string carrying a custom value.",
      "oneOf": [{ "type": "boolean" }, { "type": "string" }]
    })
  }
}

/// `scripts` 段：三个时序槽位各自的 shell 命令串
#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
#[schemars(
  description = "Lifecycle script slots: shell commands run at each phase of the version bump."
)]
pub struct ScriptsSection {
  #[schemars(description = "Shell command run before the version files are updated.")]
  pub preversion: Option<String>,
  #[schemars(
    description = "Shell command run after the version files are updated, before commit/tag."
  )]
  pub version: Option<String>,
  #[schemars(description = "Shell command run after commit/tag, before push.")]
  pub postversion: Option<String>,
}

/// `gitlab` 段：键名校验（仅 `host`、未知键专属文案）由
/// pre-pass `gitlab_host_of` 承载，结构体只管类型
#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
#[schemars(
  description = "GitLab release settings. Key-name validation lives in the pre-pass; this shape only checks types."
)]
pub struct GitlabSection {
  #[schemars(description = "Base URL of the self-hosted GitLab instance.")]
  pub host: Option<String>,
}

/// `changelog` 段形状（键集对齐消费侧 `resolve_changelog_config`；键名
/// 定制报错——遗留键 / 运行时键 / 未知键——仍在消费侧 pre-pass）
#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(
  description = "Changelog generation settings; the key set mirrors the consumer resolver."
)]
pub struct ChangelogSection {
  #[schemars(description = "Output path of the changelog file (default CHANGELOG.md).")]
  pub output: Option<String>,
  /// type 分组表：值形态见 `TypesValue`（null 跳过、`false` 删组、对象
  /// 内 `title` / `excludeScopes`——空对象 no-op）；按键深合并与声明序
  /// 语义在消费侧，本结构体仅验形（故用 BTreeMap，不保序）
  #[schemars(
    description = "Commit-type grouping table; value shape see ChangelogTypeValue. Key-wise deep merge and declaration order are consumer semantics; this shape only validates."
  )]
  pub types: Option<BTreeMap<String, Option<TypesValue>>>,
  #[schemars(
    description = "Repository used for compare links (`owner/repo` shorthand or an object); inferred from the git remote when omitted."
  )]
  pub repo: Option<RepoValue>,
  #[schemars(description = "Display-name overrides for commit scopes.")]
  pub scope_map: Option<BTreeMap<String, String>>,
  #[schemars(description = "Do not generate the contributors list.")]
  pub no_authors: Option<bool>,
  #[schemars(description = "Hide email addresses in the contributors line.")]
  pub hide_author_email: Option<bool>,
  #[schemars(description = "Exclude authors whose name matches a substring (e.g. bot accounts).")]
  pub exclude_authors: Option<Vec<String>>,
  #[schemars(description = "Changelog templates.")]
  pub templates: Option<TemplatesSection>,
  #[schemars(
    description = "Commit message used to commit the changelog file (default \"chore: update {{output}}\")."
  )]
  pub commit_message: Option<String>,
}

/// `changelog.templates` 段：形状上仅 `tagBody`（遗留键 `commitMessage` /
/// `tagMessage` 与未知键的专属报错在消费侧 pre-pass）
#[derive(Debug, Clone, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(description = "Changelog templates; only tagBody is part of the shape.")]
pub struct TemplatesSection {
  #[schemars(
    description = "Format of the version heading inside the changelog (default \"v{{newVersion}}\")."
  )]
  pub tag_body: Option<String>,
}

/// `changelog.types` 单值形态：`false`（禁用该组）或
/// `{ title?: string, excludeScopes?: string[] }`。布尔值原样透传——
/// `true` 形状层放行、由消费侧报原有文案；空对象经键缺省表达 no-op；
/// `excludeScopes` 元素非空串在形状层即拦（报错带键路径），消费侧同拦
#[derive(Debug, Clone, PartialEq)]
pub enum TypesValue {
  Bool(bool),
  Entry,
}

impl<'de> Deserialize<'de> for TypesValue {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    match Value::deserialize(deserializer)? {
      Value::Bool(b) => Ok(Self::Bool(b)),
      Value::Object(entry) => {
        if let Some(title) = entry.get("title") {
          if !title.is_string() {
            return Err(D::Error::custom("title must be a string"));
          }
        }
        if let Some(scopes) = entry.get("excludeScopes") {
          let Some(items) = scopes.as_array() else {
            return Err(D::Error::custom(
              "excludeScopes must be an array of non-empty strings",
            ));
          };
          for item in items {
            match item {
              Value::String(s) if !s.is_empty() => {}
              _ => {
                return Err(D::Error::custom(
                  "excludeScopes must be an array of non-empty strings",
                ));
              }
            }
          }
        }
        Ok(Self::Entry)
      }
      _ => Err(D::Error::custom(
        "must be false or an object { \"title\": string, \"excludeScopes\": string[] }",
      )),
    }
  }
}

impl JsonSchema for TypesValue {
  fn schema_name() -> Cow<'static, str> {
    "ChangelogTypeValue".into()
  }

  fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({
      "description": "Either false (hide this group from the changelog) or an object with an optional title overriding the group heading and an optional excludeScopes list of commit scopes to leave out of the changelog.",
      "oneOf": [
        { "const": false },
        {
          "type": "object",
          "properties": {
            "title": {
              "type": "string",
              "description": "Group heading shown in the changelog."
            },
            "excludeScopes": {
              "type": "array",
              "items": { "type": "string", "minLength": 1 },
              "description": "Commit scopes excluded from this group, matched exactly against the scope written in the commit (case-sensitive, before scopeMap). Matched non-breaking commits are left out of the changelog; breaking commits are always shown. The array replaces the built-in default wholesale (built-in: chore excludes \"deps\"); an empty array disables that built-in filtering."
            }
          }
        }
      ]
    })
  }
}

/// `changelog.repo` 联合形态（changelogen `string | RepoConfig`）：string
/// 短写经 git remote 解析；对象三可选键 provider / domain / repo（未知键
/// 报错在消费侧 pre-pass）
#[derive(Debug, Clone, PartialEq)]
pub enum RepoValue {
  Short(String),
  Explicit {
    provider: Option<String>,
    domain: Option<String>,
    repo: Option<String>,
  },
}

impl<'de> Deserialize<'de> for RepoValue {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    match Value::deserialize(deserializer)? {
      Value::String(s) => Ok(Self::Short(s)),
      Value::Object(map) => {
        let str_of = |key: &str| -> Result<Option<String>, D::Error> {
          match map.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(D::Error::custom(format!("{key} must be a string"))),
          }
        };
        Ok(Self::Explicit {
          provider: str_of("provider")?,
          domain: str_of("domain")?,
          repo: str_of("repo")?,
        })
      }
      _ => Err(D::Error::custom("must be a string or an object")),
    }
  }
}

impl JsonSchema for RepoValue {
  fn schema_name() -> Cow<'static, str> {
    "ChangelogRepo".into()
  }

  fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({
      "description": "Either the `owner/repo` shorthand string, or an object with optional provider / domain / repo.",
      "oneOf": [
        { "type": "string" },
        {
          "type": "object",
          "properties": {
            "provider": {
              "type": "string",
              "description": "Release provider name (github / gitlab / gitee / gitcode)."
            },
            "domain": {
              "type": "string",
              "description": "Self-hosted instance domain; omitted for the provider's public site."
            },
            "repo": {
              "type": "string",
              "description": "Repository in `owner/repo` form."
            }
          }
        }
      ]
    })
  }
}
