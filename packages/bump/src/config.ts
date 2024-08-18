import { loadBumpConfig } from 'bumpp'
import { ChangelogConfig, loadChangelogConfig } from 'changelogen'
import { defu } from 'defu'
import { globby } from 'globby'
import { loadConfig, presetMini } from 'esconf'
import { read, readUser } from 'rc9'
import { Config, ResolveConfig } from './types'

const getDefaultsChangeLogConfig = () =>
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
    },
  }) as Partial<ChangelogConfig>

/**
 * 合并配置项
 * @param rawConfig
 */
export const resolveConfig = async (rawConfig: Config) => {
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
    presets: [presetMini({ name: 'vbumpp' })],
  })

  const _resolveConfig = defu(
    rawConfig,
    config,
    read<ResolveConfig>({ name: 'vbumpp', flat: true }),
    readUser<ResolveConfig>({ name: 'vbumpp', flat: true }),
    {
      changelog,
      bumpp,
      accesstoken: {},
    }
  ) as ResolveConfig

  if (rawConfig.bumpp?.recursive) {
    const files = await globby('**/package.json', {
      ignore: ['**/node_modules/**'],
      cwd,
      onlyFiles: true,
    })
    files.forEach((item) => {
      _resolveConfig.bumpp.files!.push(item)
    })
  }

  _resolveConfig.bumpp.recursive = false

  // files 去重
  _resolveConfig.bumpp.files = [...new Set(_resolveConfig.bumpp.files)]

  return _resolveConfig
}

export const defineConfig = (config: Config) => config
