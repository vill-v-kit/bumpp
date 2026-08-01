//! 进度事件：对齐上游 bumpp v11 `ProgressEvent` 枚举（versionBump 执行过程中的事件类型）。
//!
//! 本模块只定义事件数据结构；事件的产生与经 ThreadsafeFunction 回传 JS 在
//! COL-15（versionBump）落地。

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
