//! Cargo 生态插件（ADR-0010）：trait 实现逐方法一行委托到能力子目录。
//! 能力本体：`version/cargo`（清单识别 + 保格式更新 + Cargo.lock 定向同步）、
//! `install/cargo`（`cargo check --workspace`）、`recursive/cargo`（清单 basename 常量）。

use std::path::Path;

use super::{
  install, recursive, version, Ecosystem, FilesError, InstallError, UpdateOutcome,
  VersionFilePlugin,
};

pub(crate) struct CargoPlugin;

impl VersionFilePlugin for CargoPlugin {
  fn matches(&self, rel_path: &Path) -> bool {
    version::cargo::matches(rel_path)
  }

  fn ecosystem(&self) -> Option<Ecosystem> {
    Some(Ecosystem::Cargo)
  }

  fn manifest_basenames(&self) -> &'static [&'static str] {
    &recursive::cargo::MANIFEST_BASENAMES
  }

  fn read_version(&self, path: &Path) -> Option<String> {
    version::cargo::read_version(path)
  }

  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
  ) -> Result<UpdateOutcome, FilesError> {
    version::cargo::update(path, rel_path, current, new)
  }

  fn install(&self, cwd: &Path) -> Option<Result<(), InstallError>> {
    Some(install::cargo::install(cwd))
  }
}
