import type {
  ChangelogOptions,
  GenerateChangelogResult,
  VersionBumpOptions,
  versionBumpInfo,
} from '@vill-v/bumpp-core'

export interface Accesstokens {
  [key: string]: string
}

/**
 * 用户配置信息：扁平镜像 `.vbumpprc.json` 形状（ADR-0013）——
 * bumpp 键居顶层，`changelog` 段并列；统一经 Rust 单一解析路径解析
 */
export interface Config extends Omit<VersionBumpOptions, 'cwd'> {
  /**
   * changelog 段配置
   */
  changelog?: ChangelogOptions
}

/**
 * 合并后的配置信息：changelog 为用户透传段（解析在 Rust 内部，
 * JS 无解析态，ADR-0013）；accesstoken 为 JS 侧加密凭证存储
 */
export interface ResolveConfig {
  /**
   * bumpp 配置信息（loadBumpConfig 合并结果）
   */
  bumpp: VersionBumpOptions
  /**
   * changelog 段（用户 overrides 透传）
   */
  changelog: ChangelogOptions
  accesstoken: Accesstokens
}

/**
 * bumpp 生成结果
 */
export type BumppResult = Awaited<ReturnType<typeof versionBumpInfo>>
/**
 * changelog 生成结果
 */
export type ChangelogResult = GenerateChangelogResult

/**
 * bumpVersion 方法返回结果
 */
export interface BumpVersion {
  /**
   * bumpp 生成结果
   */
  bumpp: BumppResult['state']
  /**
   * changelog 生成结果
   */
  changelog: ChangelogResult
  config: ResolveConfig
}
