import type { ChangelogConfig, ResolvedChangelogConfig } from 'changelogen'
import type { VersionBumpOptions, versionBumpInfo } from 'bumpp'
import { changelog } from './changelog'

/**
 * changelog 生成配置信息
 */
export interface ChangelogOptions
  extends Omit<
    Partial<ChangelogConfig>,
    'cwd' | 'tokens' | 'newVersion' | 'to' | 'from' | 'publish'
  > {}

/**
 * 用户配置信息
 */
export interface Config {
  /**
   * changelog 生成配置信息
   */
  changelog?: ChangelogOptions
  /**
   * bumpp 配置信息
   */
  bumpp?: Omit<VersionBumpOptions, 'progress' | 'cwd'>
}

/**
 * 合并后的配置信息
 */
export interface ResolveConfig {
  /**
   * changelog 生成配置信息
   */
  changelog: ResolvedChangelogConfig
  /**
   * bumpp 配置信息
   */
  bumpp: Omit<VersionBumpOptions, 'progress'>
}

/**
 * bumpp 生成结果
 */
export type BumppResult = Awaited<ReturnType<typeof versionBumpInfo>>
/**
 * changelog 生成结果
 */
export type ChangelogResult = Awaited<ReturnType<typeof changelog>>

/**
 * bumppVersion 方法返回结果
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
}
