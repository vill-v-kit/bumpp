//! npm scripts 执行：对齐上游 bumpp v11 `runNpmScript`。
//!
//! 上游只有 preversion / version / postversion 三个脚本位，由 versionBump 流程在
//! 对应步骤前后调用（编排见 COL-15）；本模块提供单个脚本的执行原语。

use std::path::Path;

use jsonc_parser::ast::Value;

use crate::exec::{run_unchecked, ExecError};
use crate::jsonc::{get_prop, is_manifest, parse};
use crate::progress::ProgressEvent;

/// 上游 `runNpmScript`：package.json 中存在对应 script 时执行 `npm run <script> --silent`。
///
/// - `ignore_scripts`、非 manifest、脚本不存在或值为 falsy → 返回 `Ok(None)`；
/// - 上游未开 throwOnError：脚本非零退出**不传播**，执行过即产出事件；
/// - package.json 读取失败（如缺失）→ 错误传播（上游 ENOENT parity）。
pub fn run_npm_script(
  cwd: &Path,
  script: &str,
  ignore_scripts: bool,
) -> Result<Option<(ProgressEvent, String)>, ExecError> {
  if ignore_scripts {
    return Ok(None);
  }
  let manifest_path = cwd.join("package.json");
  let text = std::fs::read_to_string(&manifest_path).map_err(|e| ExecError::Io {
    message: format!("读取 {} 失败：{e}", manifest_path.display()),
  })?;
  let Some(Value::Object(root)) = parse(&text) else {
    return Ok(None);
  };
  // 上游 isManifest && hasScript 双门：manifest 不合法或脚本值为 falsy 均不执行
  let should_run = is_manifest(&root)
    && get_prop(&root, "scripts")
      .and_then(|p| p.value.as_object())
      .and_then(|scripts| get_prop(scripts, script))
      .is_some_and(|p| json_truthy(&p.value));
  if !should_run {
    return Ok(None);
  }
  run_unchecked(
    NPM_BIN,
    &[
      "run".to_string(),
      script.to_string(),
      "--silent".to_string(),
    ],
    cwd,
  )?;
  Ok(Some((ProgressEvent::NpmScript, script.to_string())))
}

/// npm 可执行名：Windows 下 Rust 的 Command 不做 PATHEXT 解析，必须显式 `npm.cmd`
#[cfg(windows)]
const NPM_BIN: &str = "npm.cmd";
#[cfg(not(windows))]
const NPM_BIN: &str = "npm";

/// 上游 `Boolean(scripts[script])` 的 JS 真值语义
fn json_truthy(value: &Value) -> bool {
  match value {
    Value::NullKeyword(_) => false,
    Value::BooleanLit(b) => b.value,
    Value::StringLit(s) => !s.value.is_empty(),
    Value::NumberLit(n) => n.value.parse::<f64>().is_ok_and(|v| v != 0.0),
    Value::Object(_) | Value::Array(_) => true,
  }
}
