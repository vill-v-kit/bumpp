//! 内置进度打印的格式化函数——仿 consola 样式（success ✔ 绿 / info ℹ 蓝）。

use bumpp_core::progress::{format_line, ProgressEvent};

#[test]
fn file_updated_is_green_success_with_last_file_and_version() {
  assert_eq!(
    format_line(
      ProgressEvent::FileUpdated,
      None,
      "2.0.0",
      Some("/repo/package.json")
    ),
    "✔ Updated /repo/package.json to 2.0.0"
  );
}

#[test]
fn file_skipped_is_blue_info_with_last_file() {
  assert_eq!(
    format_line(
      ProgressEvent::FileSkipped,
      None,
      "2.0.0",
      Some("/repo/README.md")
    ),
    "ℹ /repo/README.md did not need to be updated"
  );
}

#[test]
fn git_events_match_js_progress_texts() {
  assert_eq!(
    format_line(ProgressEvent::GitCommit, None, "2.0.0", None),
    "ℹ Git commit"
  );
  assert_eq!(
    format_line(ProgressEvent::GitTag, None, "2.0.0", None),
    "ℹ Git tag"
  );
  assert_eq!(
    format_line(ProgressEvent::GitPush, None, "2.0.0", None),
    "✔ Git push"
  );
}

#[test]
fn npm_script_is_green_success_with_script_name() {
  assert_eq!(
    format_line(ProgressEvent::NpmScript, Some("preversion"), "2.0.0", None),
    "✔ Npm run preversion"
  );
}
