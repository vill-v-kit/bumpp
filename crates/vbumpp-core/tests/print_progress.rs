//! 内置进度打印的格式化函数——仿 consola 样式（success ✔ 绿 / info ℹ 蓝）。

use std::path::Path;

use vbumpp_core::progress::{format_line, ProgressEvent};

#[test]
fn file_updated_is_green_success_with_last_file_and_version() {
  // ADR-0002：cwd 内打项目相对 POSIX 路径
  assert_eq!(
    format_line(
      ProgressEvent::FileUpdated,
      None,
      "2.0.0",
      Some("/repo/package.json"),
      Path::new("/repo")
    ),
    "✔ Updated package.json to 2.0.0"
  );
}

#[test]
fn file_skipped_is_blue_info_with_last_file() {
  assert_eq!(
    format_line(
      ProgressEvent::FileSkipped,
      None,
      "2.0.0",
      Some("/repo/README.md"),
      Path::new("/repo")
    ),
    "ℹ README.md did not need to be updated"
  );
}

#[test]
fn file_outside_cwd_prints_absolute_posix() {
  // ADR-0002：cwd 外打绝对路径，分隔符同样统一 POSIX
  assert_eq!(
    format_line(
      ProgressEvent::FileUpdated,
      None,
      "2.0.0",
      Some("/elsewhere/package.json"),
      Path::new("/repo")
    ),
    "✔ Updated /elsewhere/package.json to 2.0.0"
  );
}

#[test]
fn git_events_match_js_progress_texts() {
  let cwd = Path::new("/repo");
  assert_eq!(
    format_line(ProgressEvent::GitCommit, None, "2.0.0", None, cwd),
    "ℹ Git commit"
  );
  assert_eq!(
    format_line(ProgressEvent::GitTag, None, "2.0.0", None, cwd),
    "ℹ Git tag"
  );
  assert_eq!(
    format_line(ProgressEvent::GitPush, None, "2.0.0", None, cwd),
    "✔ Git push"
  );
}

#[test]
fn script_is_green_success_with_command() {
  // ADR-0011：scripts 通用化为配置声明的 shell 命令，打印命令本体
  assert_eq!(
    format_line(
      ProgressEvent::Script,
      Some("cargo build"),
      "2.0.0",
      None,
      Path::new("/repo")
    ),
    "✔ Run cargo build"
  );
}
