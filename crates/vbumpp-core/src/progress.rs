//! 进度事件与内置打印：事件类型对齐上游 bumpp v11 `ProgressEvent`；
//! 打印样式仿 consola（ADR-0002：progress 内置 Rust，不再回传 JS）。

use std::path::Path;

use dialoguer::console::style;

/// 进度事件（上游 `ProgressEvent` 枚举；`Script` 为 ADR-0011 通用化后的形态，
/// 替代上游 `NpmScript`——npm scripts 通道已移除，事件不回传 JS，见 ADR-0002）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEvent {
  FileUpdated,
  FileSkipped,
  GitCommit,
  GitTag,
  GitPush,
  Script,
}

impl ProgressEvent {
  /// 事件的字符串值（前五项逐一对齐上游）
  pub fn as_str(self) -> &'static str {
    match self {
      Self::FileUpdated => "file updated",
      Self::FileSkipped => "file skipped",
      Self::GitCommit => "git commit",
      Self::GitTag => "git tag",
      Self::GitPush => "git push",
      Self::Script => "script",
    }
  }
}

/// 事件 → CLI 输出字符串（仿 consola 样式：success ✔ 绿 / info ℹ 蓝；
/// 非 TTY 时 console 自动降级为无 ANSI 纯文本）。
/// `file` 为本次事件对应的文件路径（FileUpdated / FileSkipped 的最后一个，
/// 绝对原生形态的存储值）——打印层按显示路径规则转换（ADR-0002：cwd 内相对、
/// cwd 外绝对、一律 POSIX），存储与 API 返回值不受影响
pub fn format_line(
  event: ProgressEvent,
  script: Option<&str>,
  new_version: &str,
  file: Option<&str>,
  cwd: &Path,
) -> String {
  let file = file.map(|f| crate::display::path(cwd, Path::new(f)));
  match event {
    ProgressEvent::FileUpdated => {
      format!(
        "{} Updated {} to {new_version}",
        style("✔").green(),
        file.as_deref().unwrap_or_default()
      )
    }
    ProgressEvent::FileSkipped => {
      format!(
        "{} {} did not need to be updated",
        style("ℹ").blue(),
        file.as_deref().unwrap_or_default()
      )
    }
    ProgressEvent::GitCommit => format!("{} Git commit", style("ℹ").blue()),
    ProgressEvent::GitTag => format!("{} Git tag", style("ℹ").blue()),
    ProgressEvent::GitPush => format!("{} Git push", style("✔").green()),
    ProgressEvent::Script => {
      format!("{} Run {}", style("✔").green(), script.unwrap_or_default())
    }
  }
}
