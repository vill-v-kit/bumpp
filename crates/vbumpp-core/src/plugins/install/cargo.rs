//! Cargo 生态 install 适配（ADR-0007）：`cargo check --workspace`——ADR-0003 点名的
//! 兜底刷新方式：校验 Cargo.lock 定向同步结果、兜底刷新遗漏，并验证 workspace
//! 仍可编译。Cargo 为单一工具链，无需 JavaScript 生态意义上的"检测"。

use std::path::Path;

use crate::effects::{Effects, RealEffects};
use crate::plugins::InstallError;

/// 经效应边界执行 `cargo check --workspace`；失败即报错（发版一致性优先）
pub fn install(cwd: &Path) -> Result<(), InstallError> {
  install_with(&RealEffects, cwd)
}

/// `install` 的效应注入形态
pub fn install_with(eff: &dyn Effects, cwd: &Path) -> Result<(), InstallError> {
  eff
    .run(
      "cargo",
      &["check".to_string(), "--workspace".to_string()],
      cwd,
    )
    .map_err(|e| InstallError {
      message: e.to_string(),
    })
}
