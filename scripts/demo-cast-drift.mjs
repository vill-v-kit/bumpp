#!/usr/bin/env node
/**
 * 首页滚动演示 cast 产物漂移校验（COL-93）：ADR-0036「本地生成提交 + CI 漂移
 * 校验」的防腐腿。重跑四段演示采集、与提交进 website 的 cast TS 模块 diff，
 * 不一致即失败——CLI 输出变更后演示腐烂被显式暴露，而非静默发生。
 *
 * 采集脚本的输出即唯一真相，提交产物只是它的快照；手改产物无效，改输出后
 * 更新演示是一步显式操作（再生成 + 提交）。
 *
 * 用法：
 *   node scripts/demo-cast-drift.mjs
 *     1. pnpm --filter website capture:home-demo-cast 重跑采集（原地重写产物，
 *        需已构建 target/release/vbumpp）
 *     2. git diff --exit-code 断言产物与提交内容一致（diff 原文进日志）
 *
 * 测试 stub（scripts/demo-cast-drift.test.mjs）：DEMO_CAST_CAPTURE_CMD 覆盖
 * 采集命令（真实采集依赖 release 二进制，契约测试不进那条路）。
 *
 * 退出码契约：0 一致；1 漂移 / 采集失败；2 环境错误（不在 git 仓库内）。
 */
import { execFileSync, spawnSync } from 'node:child_process'

const CAST_PATH = 'website/app/(home)/demo-casts.ts'
const CAPTURE_CMD = process.env.DEMO_CAST_CAPTURE_CMD ?? 'pnpm --filter website capture:home-demo-cast'

let root
try {
  root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim()
} catch {
  console.error('error: 不在 git 仓库内——漂移校验依赖 git diff 比对提交产物')
  process.exit(2)
}

// 1. 重跑采集（采集脚本自身有完整性门禁与二进制存在检查，失败在此原样暴露）
const capture = spawnSync(CAPTURE_CMD, { cwd: root, shell: true, stdio: 'inherit' })
if (capture.status !== 0) {
  console.error(
    `::error::首页演示 cast 采集失败（exit ${capture.status ?? 'signal'}）——` +
      '先确认 target/release/vbumpp 已构建（cargo build --release -p vbumpp）；' +
      '采集脚本依赖 macOS BSD script(1)，仅 macOS 可跑',
  )
  process.exit(1)
}

// 2. diff 提交产物——漂移即红
const diff = spawnSync('git', ['diff', '--exit-code', '--', CAST_PATH], { cwd: root, stdio: 'inherit' })
if (diff.status !== 0) {
  console.error(
    `::error::首页演示 cast 产物漂移——${CAST_PATH} 与采集脚本输出不一致\n` +
      `${CAST_PATH} 是生成物，手改无效；唯一真相是采集脚本的输出。\n` +
      '本地再生成：cargo build --release -p vbumpp && pnpm --filter website capture:home-demo-cast，' +
      '然后提交产物变更。',
  )
  process.exit(1)
}

console.log('demo-cast-drift: 产物与采集输出一致')
