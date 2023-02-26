import { Config, ResolveConfig } from './types'
import { defu } from 'defu'
import { ChangelogConfig } from 'changelogen'
import { loadConfig } from 'c12'

const changeLogConfigDefaults: ChangelogConfig = {
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
    BreakingChange: { title: '⚠️ 破坏性改动' },
  },
  from: '',
  to: '',
  output: 'CHANGELOG.md',
  scopeMap: {},
  newVersion: '',
  github: '',
  cwd: '',
}

export const resolveConfig = async (rawConfig: Config) => {
  const { globby } = await import('globby')
  const { config } = await loadConfig<ResolveConfig>({
    name: 'vbumpp',
    globalRc: true,
    defaults: {
      changelog: changeLogConfigDefaults,
      bumpp: {
        cwd: process.cwd(),
        files: ['package.json', 'package-lock.json'],
      },
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
