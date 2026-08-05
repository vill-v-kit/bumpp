import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    projects: [
      'napi/*',
      'npm/*',
      // 根级 scripts/ 的独立测试项目（如 publish-guard 的 CLI 契约测试）
      { test: { include: ['scripts/**/*.test.mjs'] } },
    ],
  },
})
