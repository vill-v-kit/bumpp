/**
 * changelog 系符号（changelogen 使用面的 Rust 重写，ADR-0012）——
 * 实现统一在 @vill-v/bumpp-core，本文件仅显式收窄再导出
 * （原子路径 @vill-v/bumpp/changelog，专供兄弟包）
 */
export {
  generateChangelog,
  getCurrentGitBranch,
  getGitDiff,
  getLastGitTag,
  resolveRepoConfig,
} from '@vill-v/bumpp-core'
export type {
  ChangelogOptions,
  GenerateChangelogResult,
  GitAuthor,
  RawCommit,
  RepoConfig,
} from '@vill-v/bumpp-core'
