//! 显示路径（ADR-0023）：打印到控制台的路径的统一形态——cwd 之内打相对路径，
//! cwd 之外打绝对路径，一律 POSIX 分隔符（`/`）。只约束显示层；存储与 API
//! 返回值（`updated_files` / napi `updatedFiles` 等）保持绝对原生路径不变。

use std::path::Path;

/// cwd 锚定的显示路径：`path` 在 `cwd` 之内打相对路径（即 cwd 本身打 `.`），
/// 之外打绝对路径，分隔符一律转为 `/`。`strip_prefix` 失败即落绝对分支——
/// token 存储 / 全局配置（home 目录）、`..` 逃逸的显式参数等 cwd 外路径走此支
pub fn path(cwd: &Path, path: &Path) -> String {
  match path.strip_prefix(cwd) {
    Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
    Ok(rel) => posix(rel),
    Err(_) => posix(path),
  }
}

/// 无锚点的显示路径：仅统一分隔符为 `/`——路径本就在项目外（token 存储），
/// 或本来就是相对形态（插件读写的 rel_path），无 cwd 可锚
pub fn posix(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
  use super::*;

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
    assert_eq!(path(Path::new("/repo"), Path::new("package.json")), "package.json");
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
}
