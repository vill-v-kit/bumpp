//! 文本兜底插件（ADR-0010）：仅有版本更新能力（文本模板替换），无清单、
//! 无版本解析、无 install 适配。必须处于链尾（matches 恒真）。

use std::path::Path;

use super::{version, Ecosystem, FilesError, InstallError, UpdateOutcome, VersionFilePlugin};

pub(crate) struct TextPlugin;

impl VersionFilePlugin for TextPlugin {
  fn matches(&self, rel_path: &Path) -> bool {
    version::text::matches(rel_path)
  }

  /// 兜底通道不归属任何生态（不贡献 install 触发，ADR-0008）
  fn ecosystem(&self) -> Option<Ecosystem> {
    None
  }

  /// 兜底通道无清单概念
  fn manifest_basenames(&self) -> &'static [&'static str] {
    &[]
  }

  /// 兜底通道无版本解析能力（ADR-0009）
  fn read_version(&self, _path: &Path) -> Option<String> {
    None
  }

  fn update(
    &self,
    path: &Path,
    rel_path: &Path,
    current: &str,
    new: &str,
    _cwd: &Path,
  ) -> Result<UpdateOutcome, FilesError> {
    version::text::update(path, rel_path, current, new)
  }

  /// 兜底通道无 install 适配
  fn install(&self, _cwd: &Path) -> Option<Result<(), InstallError>> {
    None
  }
}
