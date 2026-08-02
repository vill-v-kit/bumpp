//! Node 生态插件（ADR-0010）：trait 实现逐方法一行委托到能力子目录。
//! 能力本体：`version/node`（清单识别 + 保格式更新）、`install/node`
//! （PM 检测 + `<pm> install`）、`recursive/node`（清单 basename 常量）。

use std::path::Path;

use super::{
  install, recursive, version, Ecosystem, FilesError, InstallError, UpdateOutcome,
  VersionFilePlugin,
};

pub(crate) struct NodePlugin;

impl VersionFilePlugin for NodePlugin {
  fn matches(&self, rel_path: &Path) -> bool {
    version::node::matches(rel_path)
  }

  fn ecosystem(&self) -> Option<Ecosystem> {
    Some(Ecosystem::Node)
  }

  fn manifest_basenames(&self) -> &'static [&'static str] {
    &recursive::node::MANIFEST_BASENAMES
  }

  fn read_version(&self, path: &Path) -> Option<String> {
    version::node::read_version(path)
  }

  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
  ) -> Result<UpdateOutcome, FilesError> {
    version::node::update(path, rel_path, current, new)
  }

  fn install(&self, cwd: &Path) -> Option<Result<(), InstallError>> {
    Some(install::node::install(cwd))
  }
}
