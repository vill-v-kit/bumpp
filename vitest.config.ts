import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    projects: [
      'napi/*',
      'npm/*',
      // 根级 scripts/ 的独立测试项目（如 publish-guard 的 CLI 契约测试）
      { test: { include: ['scripts/**/*.test.mjs'] } },
      // website 的纯逻辑单元测试（首页演示进度控制器，不碰 Next/React 运行时）
      { test: { include: ['website/**/*.test.ts'] } },
    ],
  },
})
