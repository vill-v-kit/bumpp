import { defu } from 'defu'
import { ChangelogConfig, loadChangelogConfig } from 'changelogen'
import { loadConfig } from 'c12'
import { globby } from 'globby'
import { loadBumpConfig } from 'bumpp'
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
  const changelog = await loadChangelogConfig(cwd)

  const bumpp = await loadBumpConfig({
    cwd,
    files: ['package.json', 'package-lock.json'],
  })

  const { config } = await loadConfig<ResolveConfig>({
    name: 'vbumpp',
    globalRc: true,
    defaults: {
      changelog: defu(changelog, getDefaultsChangeLogConfig()),
      bumpp,
    },
  })

  const _resolveConfig = defu(rawConfig, config) as ResolveConfig

  if (rawConfig.bumpp?.recursive) {
    const files = await globby('**/package.json', {
      ignore: ['**/node_modules/**'],
      cwd: process.cwd(),
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
