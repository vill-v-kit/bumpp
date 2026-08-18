//! 显示路径单元测试：cwd 锚定与 POSIX 分隔符形态矩阵。

use std::path::Path;

use vbumpp_core::display::{path, posix};

#[test]
fn inside_cwd_prints_relative_posix() {
  assert_eq!(
    path(
      Path::new("/repo"),
      Path::new("/repo/packages/sub/package.json")
    ),
    "packages/sub/package.json"
  );
}

#[test]
fn cwd_itself_prints_dot() {
  assert_eq!(path(Path::new("/repo"), Path::new("/repo")), ".");
}

#[test]
fn outside_cwd_prints_absolute() {
  assert_eq!(
    path(Path::new("/repo"), Path::new("/home/u/.vbumpp/tokens.bin")),
    "/home/u/.vbumpp/tokens.bin"
  );
}

#[test]
fn relative_input_stays_relative() {
  assert_eq!(
    path(Path::new("/repo"), Path::new("package.json")),
    "package.json"
  );
}

#[test]
fn separators_always_posix() {
  // unix 上 `\` 是普通文件名字符，可模拟 Windows 形态字符串；
  // Windows 下 strip_prefix 原生分隔符命中后余量本为相对段，replace 为幂等
  assert_eq!(
    path(Path::new("/repo"), Path::new("C:\\repo\\package.json")),
    "C:/repo/package.json"
  );
  assert_eq!(posix(Path::new("a\\b\\c")), "a/b/c");
  assert_eq!(posix(Path::new("packages/sub")), "packages/sub");
}
