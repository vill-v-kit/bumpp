//! 显示路径（ADR-0002）：打印到控制台的路径的统一形态——cwd 之内打相对路径，
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
