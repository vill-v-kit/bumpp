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
  const { config } = await loadConfig<ResolveConfig>({
    defaults: {
      changelog: changeLogConfigDefaults,
      bumpp: {
        cwd: process.cwd(),
      },
    },
  })

  return defu(rawConfig, config) as ResolveConfig
}
