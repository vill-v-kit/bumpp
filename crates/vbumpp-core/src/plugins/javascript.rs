//! JavaScript 生态插件（ADR-0007）：trait 实现逐方法一行委托到能力子目录。
//! 能力本体：`version/javascript`（清单识别 + 保格式更新）、`install/javascript`
//! （PM 检测 + `<pm> install`）、`recursive/javascript`（清单 basename 常量）。

use std::path::Path;

use super::{
  install, recursive, version, Ecosystem, FilesError, InstallError, UpdateOutcome,
  VersionFilePlugin,
};

pub(crate) struct JavaScriptPlugin;

impl VersionFilePlugin for JavaScriptPlugin {
  fn matches(&self, rel_path: &Path) -> bool {
    version::javascript::matches(rel_path)
  }

  fn ecosystem(&self) -> Option<Ecosystem> {
    Some(Ecosystem::JavaScript)
  }

  fn manifest_basenames(&self) -> &'static [&'static str] {
    &recursive::javascript::MANIFEST_BASENAMES
  }

  fn read_version(&self, path: &Path) -> Option<String> {
    version::javascript::read_version(path)
  }

  /// 本通道错误消息只用 rel_path（显示路径的相对形态，ADR-0002），无需 cwd 锚点
  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
    _cwd: &Path,
  ) -> Result<UpdateOutcome, FilesError> {
    version::javascript::update(path, rel_path, current, new)
  }

  fn install(&self, cwd: &Path) -> Option<Result<(), InstallError>> {
    Some(install::javascript::install(cwd))
  }
}
