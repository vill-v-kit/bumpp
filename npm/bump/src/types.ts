import type { BumpState, BumpVersionResult, GenerateChangelogResult } from '@vill-v/bumpp-core'

export type { BumpState, BumpVersionResult, GenerateChangelogResult }

/**
 * changelog 段单个 type 分组
 */
export interface ChangelogTypeEntry {
  title: string
}

/**
 * changelog 段 `repo` 的对象形态
 */
export interface RepoConfig {
  provider?: string
  domain?: string
  repo?: string
}

/**
 * changelog 段配置（ADR-0013 支持键集；Rust 单一解析路径内部消费，JS 仅透传）
 */
export interface ChangelogOptions {
  output?: string
  types?: Record<string, ChangelogTypeEntry | false>
  repo?: string | RepoConfig
  scopeMap?: Record<string, string>
  noAuthors?: boolean
  hideAuthorEmail?: boolean
  excludeAuthors?: string[]
  templates?: {
    tagBody?: string
  }
  commitMessage?: string
}

/**
 * 用户配置信息：扁平镜像配置文件形状（ADR-0013）——bumpp 键居顶层，
 * `changelog` / `gitlab` 段并列；统一经 Rust 单一解析路径解析
 * （overrides > 项目 .vbumpprc.* > 全局 ~/.vbumpp/config.* > 内建默认）
 */
export interface Config {
  release?: string
  files?: string[]
  commit?: boolean | string
  tag?: boolean | string
  push?: boolean
  sign?: boolean
  all?: boolean
  noVerify?: boolean
  confirm?: boolean
  ignoreScripts?: boolean
  install?: boolean
  execute?: string
  scripts?: {
    preversion?: string
    version?: string
    postversion?: string
  }
  preid?: string
  currentVersion?: string
  recursive?: boolean
  configFilePath?: string
  /**
   * changelog 段配置
   */
  changelog?: ChangelogOptions
  /**
   * gitlab 段（ADR-0014）：自建实例 host，缺省 https://gitlab.com
   */
  gitlab?: {
    host?: string
  }
}

/**
 * bumpVersion 返回结果（ADR-0014 收缩后）：版本状态 + changelog（无 tag 时缺省）。
 * 明文 token 不出 Rust 边界，`config` 字段已随重写移除
 */
export type BumpVersion = BumpVersionResult
