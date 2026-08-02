//! 包管理器检测：对齐上游 package-manager-detector 的默认行为（ADR-0006）。
//!
//! 逐级向上爬目录（目录为外层循环）；每级目录内按上游默认策略顺序
//! lockfile → packageManager 字段检测，字段值不识别时 fall through。
//! agent / lockfile 名单对齐上游 `AGENTS` / `LOCKS` 常量。
//!
//! 边界：唯一消费方是 `version_bump` 的 --install（install 命令为
//! `<agent> install`，无需上游 COMMANDS 映射）；第二消费方出现时按
//! ADR-0004 同款流程拆 `crates/package-manager`。

use std::error::Error;
use std::fmt;
use std::path::Path;

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

/// 上游 `detect`（默认策略）：自 cwd 逐级上爬；每级先 lockfile 后 packageManager 字段
pub fn detect_package_manager(cwd: &Path) -> Result<&'static str, PmError> {
  for dir in cwd.ancestors() {
    for (lock, agent) in LOCKS {
      if dir.join(lock).exists() {
        return Ok(agent);
      }
    }
    if let Some(agent) = detect_from_package_manager_field(dir) {
      return Ok(agent);
    }
  }
  Err(PmError {
    message: "Could not detect package manager, failed to run npm install".to_string(),
  })
}

/// 上游 `packageManager-field` 策略：JSONC 容错解析；字段值不识别返回 None
/// （fall through 到其他策略，而非误判为 npm）
fn detect_from_package_manager_field(dir: &Path) -> Option<&'static str> {
  let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
  let jsonc_parser::ast::Value::Object(root) = crate::jsonc::parse(&text)? else {
    return None;
  };
  let field = crate::jsonc::get_prop(&root, "packageManager")?
    .value
    .as_string_lit()?;
  let name = field.value.split('@').next()?;
  AGENTS.iter().copied().find(|agent| *agent == name)
}
