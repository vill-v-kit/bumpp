//! JavaScript 生态 install 适配（ADR-0007）：检测包管理器后执行 `<pm> install`。
//!
//! 检测对齐上游 package-manager-detector 的默认行为（ADR-0006）：逐级向上爬
//! 目录（目录为外层循环）；每级目录内依次检测 lockfile → 顶层 packageManager
//! 字段 → devEngines.packageManager 声明，值不识别时 fall through。agent /
//! lockfile 名单对齐上游 `AGENTS` / `LOCKS` 常量；devEngines 声明只消费
//! `name`，不复刻 Corepack 的版本、onFail 或冲突校验语义。install 命令对
//! 名单内 agent 恒为 `<agent> install`（无需上游 COMMANDS 映射）。

use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::effects::{Effects, RealEffects};
use crate::plugins::InstallError;

/// JavaScript 适配入口：detect → `<pm> install`（上游 `options.install` 语义）
pub fn install(cwd: &Path) -> Result<(), InstallError> {
  install_with(&RealEffects, cwd)
}

/// `install` 的效应注入形态（spawn 经效应边界；PM 检测为只读计算）
pub fn install_with(eff: &dyn Effects, cwd: &Path) -> Result<(), InstallError> {
  let pm = detect_package_manager(cwd).map_err(|e| InstallError {
    message: e.to_string(),
  })?;
  eff
    .run(pm, &["install".to_string()], cwd)
    .map_err(|e| InstallError {
      message: e.to_string(),
    })
}

#[derive(Debug)]
pub struct PmError {
  message: String,
}

impl fmt::Display for PmError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl Error for PmError {}

/// 上游 `LOCKS`（顺序重要：more specific one comes first）
const LOCKS: [(&str, &str); 11] = [
  ("aube-lock.yaml", "aube"),
  ("aube-workspace.yaml", "aube"),
  ("bun.lock", "bun"),
  ("bun.lockb", "bun"),
  ("deno.lock", "deno"),
  ("nub.lock", "nub"),
  ("pnpm-lock.yaml", "pnpm"),
  ("pnpm-workspace.yaml", "pnpm"),
  ("yarn.lock", "yarn"),
  ("package-lock.json", "npm"),
  ("npm-shrinkwrap.json", "npm"),
];

/// 上游 `AGENTS` 的 name 部分（`packageManager` 字段值形如 `<name>@<version>`）
const AGENTS: [&str; 7] = ["npm", "yarn", "pnpm", "bun", "deno", "nub", "aube"];

/// 上游 `detect`（默认策略）：自 cwd 逐级上爬；每级先 lockfile，再 package.json
/// 声明（顶层 packageManager → devEngines.packageManager）
pub fn detect_package_manager(cwd: &Path) -> Result<&'static str, PmError> {
  for dir in cwd.ancestors() {
    for (lock, agent) in LOCKS {
      if dir.join(lock).exists() {
        return Ok(agent);
      }
    }
    if let Some(agent) = detect_from_package_json(dir) {
      return Ok(agent);
    }
  }
  Err(PmError {
    message: "Could not detect package manager, failed to run npm install".to_string(),
  })
}

/// 每级目录的 package.json 声明检测：JSONC 容错解析；两种声明均不识别返回
/// None（fall through 到其他目录，而非误判为 npm）。同级顺序：顶层
/// packageManager → devEngines.packageManager（ADR-0006）
fn detect_from_package_json(dir: &Path) -> Option<&'static str> {
  let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
  let jsonc_parser::ast::Value::Object(root) = crate::jsonc::parse(&text)? else {
    return None;
  };
  detect_from_package_manager_field(&root).or_else(|| detect_from_dev_engines_field(&root))
}

/// 支持名单匹配：name 不在上游 `AGENTS` 内返回 None（宽容回退）
fn known_agent(name: &str) -> Option<&'static str> {
  AGENTS.iter().copied().find(|agent| *agent == name)
}

/// 上游 `packageManager-field` 策略：字段值 `<name>@<version>` 的 name 部分
fn detect_from_package_manager_field(root: &jsonc_parser::ast::Object) -> Option<&'static str> {
  let field = crate::jsonc::get_prop(root, "packageManager")?
    .value
    .as_string_lit()?;
  known_agent(field.value.split('@').next()?)
}

/// devEngines.packageManager 声明（ADR-0006）：只接受单个对象且只消费
/// `name`（version / onFail / 未知属性不参与调度）；字符串、数组、缺失或
/// 非字符串 name、未知名称一律宽容回退 None
fn detect_from_dev_engines_field(root: &jsonc_parser::ast::Object) -> Option<&'static str> {
  let dev_engines = crate::jsonc::get_prop(root, "devEngines")?
    .value
    .as_object()?;
  let pm = crate::jsonc::get_prop(dev_engines, "packageManager")?
    .value
    .as_object()?;
  let name = crate::jsonc::get_prop(pm, "name")?.value.as_string_lit()?;
  known_agent(&name.value)
}
