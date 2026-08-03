//! changelog 域（ADR-0012）：changelogen 使用面的 Rust 重写。
//! 编排与对外 API 收于此根部；能力子目录：配置段解析（`config`）、
//! markdown 生成（`markdown`）、gitmoji 数据表（`gitmoji`）。

pub mod config;
pub mod gitmoji;
pub mod markdown;

use crate::commits::{parse_display_commit, DisplayCommit};
use crate::git::RawCommit;

use crate::changelog::config::ChangelogConfig;
use crate::changelog::markdown::ReleaseRange;

/// 引擎管线：RawCommit → 展示层解析 → 类型过滤（config.types 键）→
/// `chore(deps)` 过滤（chore + scope deps + 非 breaking；scope 为 scopeMap
/// 应用后的值——与原 JS 同一位点的 quirk 保持一致）→ markdown 生成。
/// 全程纯函数、无 IO、无网络。
pub fn render_changelog(
  raw_commits: &[RawCommit],
  config: &ChangelogConfig,
  range: &ReleaseRange,
) -> String {
  let commits: Vec<DisplayCommit> = raw_commits
    .iter()
    .filter_map(|raw| parse_display_commit(raw, &config.scope_map))
    .filter(|c| config.types.iter().any(|(n, _)| n == &c.commit_type))
    .filter(|c| !(c.commit_type == "chore" && c.scope == "deps" && !c.is_breaking))
    .collect();
  markdown::generate_markdown(&commits, config, range)
}
