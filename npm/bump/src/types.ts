// 配置类型以 napi 边界结构体为单一事实源，TS 类型机械生成——
// 手写门面 interface 已删除，此处仅再导出生成类型并保留旧名兼容别名
import type { BumpConfig, BumpVersionResult, ChangelogSection } from '@vill-v/bumpp-core'

export type {
  BumpConfig,
  BumpState,
  BumpVersionResult,
  ChangelogSection,
  ChangelogTypeEntry,
  ChangelogTypes,
  ChangelogTypeValue,
  GenerateChangelogResult,
  GitlabSection,
  RepoConfig,
  ScriptsSection,
  TemplatesSection,
} from '@vill-v/bumpp-core'

/**
 * 用户配置信息：napi 生成 `BumpConfig` 的别名——扁平镜像配置文件形状
 *，另含 overrides 专用机制键 `configFilePath`；统一经 Rust
 * 单一解析路径解析（overrides > 项目 .vbumpprc.* > 全局 ~/.vbumpp/config.* > 内建默认）
 */
export type Config = BumpConfig

/**
 * changelog 段配置：napi 生成 `ChangelogSection` 的别名
 */
export type ChangelogOptions = ChangelogSection

/**
 * bumpVersion 返回结果（收缩后）：版本状态 + changelog（无 tag 时缺省）。
 * 明文 token 不出 Rust 边界，`config` 字段已随重写移除
 */
export type BumpVersion = BumpVersionResult
