import { loadBumpConfig, versionFileManifestGlobs } from '@vill-v/bumpp-core'
import { type ChangelogConfig, loadChangelogConfig } from 'changelogen'
import { defu } from 'defu'
import { loadConfig } from '@esconf/core'
import { presetMini } from '@esconf/preset-mini'
import { consola } from 'consola'
import type { Config, ResolveConfig } from './types'
import { readTokenStore } from './accesstoken'

const getDefaultsChangeLogConfig = (): Partial<ChangelogConfig> =>
  ({
    types: {
      feat: { title: '🚀 特性' },
      perf: { title: '🔥 性能优化' },
      fix: { title: '🩹 修复' },
      refactor: { title: '💅 重构' },
      examples: { title: '🏀 示例' },
      docs: { title: '📖 文档' },
      chore: { title: '🏡 框架' },
      build: { title: '📦 打包' },
      test: { title: '✅ 测试' },
      BreakingChange: { title: '🚨 破坏性改动' },
      style: { title: '🎨 样式' },
    },
  }) as Partial<ChangelogConfig>

/**
 * 合并配置项
 * @param rawConfig
 */
export const resolveConfig = async (rawConfig: Config): Promise<ResolveConfig> => {
  const cwd = process.cwd()
  const changelog = await loadChangelogConfig(cwd, getDefaultsChangeLogConfig())

  const bumpp = await loadBumpConfig({
    cwd,
    files: ['package.json', 'package-lock.json'],
    commit: true,
    tag: true,
    push: true,
    confirm: false,
    ignoreScripts: false,
    noVerify: false,
  })

  const { config } = await loadConfig<ResolveConfig>({
    presets: [
      presetMini({
        name: 'vbumpp',
        configName: 'config',
      }),
    ],
  })

  const _resolveConfig = defu(rawConfig, config, {
    changelog,
    bumpp,
    accesstoken: {},
  }) as ResolveConfig

  // 获取独立二进制存储中的 token
  _resolveConfig.accesstoken = {}
  try {
    _resolveConfig.accesstoken = await readTokenStore()
  } catch {
    consola.warn('token 存储文件读取失败（设备或用户已变更），请重新执行 vbumpp token set <name>')
  }

  if (rawConfig.bumpp?.recursive) {
    // -r 整树收集（ADR-0003 opt-in）：模式表由 core 插件链聚合导出——
    // 生态清单知识的单一事实源，未来生态在 core 链上落插件即自动纳入；
    // 展开与 IGNORED_DIRS 过滤由 core normalize_files 统一承担
    _resolveConfig.bumpp.files!.push(...versionFileManifestGlobs())
  }

  _resolveConfig.bumpp.recursive = false

  // files 去重
  _resolveConfig.bumpp.files = [...new Set(_resolveConfig.bumpp.files)]

  return _resolveConfig
}

export const defineConfig = (config: Config): Config => config
