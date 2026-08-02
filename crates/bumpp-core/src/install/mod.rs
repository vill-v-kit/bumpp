//! install 生态适配（ADR-0008）：按本次 bump 实际更新的生态，触发对应的安装/校验。
//!
//! 每生态一文件：`node`（上游 package-manager-detector parity + `<pm> install`）、
//! `cargo`（`cargo check --workspace`，ADR-0003 点名的兜底校验）。maven / gradle
//! 等未来生态以新文件加入。
//!
//! 触发语义（有意偏离上游，ADR-0008）：仅当本次 bump 有文件被实际更新
//! （FileUpdated）时，按更新文件所属生态集合触发；零生态命中（仅 Text 兜底
//! 通道的文件被更新）回退 node——与上游 `--install` 行为一致。

use std::error::Error;
use std::fmt;
use std::path::Path;

pub mod cargo;
pub mod node;

pub use crate::files::Ecosystem;

#[derive(Debug)]
pub struct InstallError {
  message: String,
}

impl fmt::Display for InstallError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl Error for InstallError {}

/// 更新文件清单 → 待触发的生态集合（固定顺序 Node → Cargo；零命中回退 Node）
pub fn resolve_ecosystems(updated_files: &[String]) -> Vec<Ecosystem> {
  let mut ecosystems = Vec::new();
  for eco in [Ecosystem::Node, Ecosystem::Cargo] {
    if updated_files
      .iter()
      .any(|f| crate::files::ecosystem_of(Path::new(f)) == Some(eco))
    {
      ecosystems.push(eco);
    }
  }
  if ecosystems.is_empty() {
    ecosystems.push(Ecosystem::Node);
  }
  ecosystems
}

/// 按生态集合依次执行各生态的 install 适配
pub fn run_installs(cwd: &Path, updated_files: &[String]) -> Result<(), InstallError> {
  for eco in resolve_ecosystems(updated_files) {
    match eco {
      Ecosystem::Node => node::install(cwd)?,
      Ecosystem::Cargo => cargo::install(cwd)?,
    }
  }
  Ok(())
}
