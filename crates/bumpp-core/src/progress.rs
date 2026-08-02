//! 进度事件与内置打印：事件类型对齐上游 bumpp v11 `ProgressEvent`；
//! 打印样式仿 consola（ADR-0002：progress 内置 Rust，不再回传 JS）。

/// 上游 `ProgressEvent` 枚举，字符串值逐一对齐
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEvent {
  FileUpdated,
  FileSkipped,
  GitCommit,
  GitTag,
  GitPush,
  NpmScript,
}

impl ProgressEvent {
  /// 上游枚举的字符串值
  pub fn as_str(self) -> &'static str {
    match self {
      Self::FileUpdated => "file updated",
      Self::FileSkipped => "file skipped",
      Self::GitCommit => "git commit",
      Self::GitTag => "git tag",
      Self::GitPush => "git push",
      Self::NpmScript => "npm script",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn event_strings_match_upstream() {
    assert_eq!(ProgressEvent::FileUpdated.as_str(), "file updated");
    assert_eq!(ProgressEvent::FileSkipped.as_str(), "file skipped");
    assert_eq!(ProgressEvent::GitCommit.as_str(), "git commit");
    assert_eq!(ProgressEvent::GitTag.as_str(), "git tag");
    assert_eq!(ProgressEvent::GitPush.as_str(), "git push");
    assert_eq!(ProgressEvent::NpmScript.as_str(), "npm script");
  }
}

use dialoguer::console::style;

/// 事件 → CLI 输出字符串（仿 consola 样式：success ✔ 绿 / info ℹ 蓝；
/// 非 TTY 时 console 自动降级为无 ANSI 纯文本）。
/// `file` 为本次事件对应的文件路径（FileUpdated / FileSkipped 的最后一个）。
pub fn format_line(
  event: ProgressEvent,
  script: Option<&str>,
  new_version: &str,
  file: Option<&str>,
) -> String {
  match event {
    ProgressEvent::FileUpdated => {
      format!(
        "{} Updated {} to {new_version}",
        style("✔").green(),
        file.unwrap_or_default()
      )
    }
    ProgressEvent::FileSkipped => {
      format!(
        "{} {} did not need to be updated",
        style("ℹ").blue(),
        file.unwrap_or_default()
      )
    }
    ProgressEvent::GitCommit => format!("{} Git commit", style("ℹ").blue()),
    ProgressEvent::GitTag => format!("{} Git tag", style("ℹ").blue()),
    ProgressEvent::GitPush => format!("{} Git push", style("✔").green()),
    ProgressEvent::NpmScript => {
      format!(
        "{} Npm run {}",
        style("✔").green(),
        script.unwrap_or_default()
      )
    }
  }
}

/// 内置打印到 stdout（与 dialoguer prompt / printSummary 同通道）
pub(crate) fn print_line(
  event: ProgressEvent,
  script: Option<&str>,
  new_version: &str,
  file: Option<&str>,
) {
  println!("{}", format_line(event, script, new_version, file));
}
