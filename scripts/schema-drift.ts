#!/usr/bin/env node
/**
 * schema 产物漂移校验（COL-104）：ADR-0037「本地生成提交 + CI 漂移校验」的
 * 防腐腿（与 demo cast 防腐同款）。重跑两处 schema 产物的再生、与提交进仓库
 * 的产物 diff，不一致即失败——配置形状（vbumpp-core 结构体，单一事实源）变更
 * 后忘再生、或手改产物，都被显式暴露而非静默发生。
 *
 * 用法：
 *   node scripts/schema-drift.ts
 *     1. 重跑再生（默认 node scripts/regen-schema.ts，需已构建 vbumpp 二进制）
 *     2. git diff --exit-code 断言产物与提交内容一致（diff 原文进日志）
 *
 * 测试 stub（scripts/schema-drift.test.ts）：SCHEMA_REGEN_CMD 覆盖再生命令
 * （真实再生依赖二进制，契约测试不进那条路）。
 *
 * 退出码契约：0 一致；1 漂移 / 再生失败；2 环境错误（不在 git 仓库内）。
 */
import { execFileSync, spawnSync } from 'node:child_process'

const ARTIFACTS = ['npm/bump/vbumpprc.schema.json', 'website/public/vbumpprc.schema.json']
const REGEN_CMD = process.env.SCHEMA_REGEN_CMD ?? 'node scripts/regen-schema.ts'

let root: string
try {
  root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim()
} catch {
  console.error('error: 不在 git 仓库内——漂移校验依赖 git diff 比对提交产物')
  process.exit(2)
}

// 1. 重跑再生（再生脚本自身有二进制解析与纯 JSON 门禁，失败在此原样暴露）
const regen = spawnSync(REGEN_CMD, { cwd: root, shell: true, stdio: 'inherit' })
if (regen.status !== 0) {
  console.error(
    `::error::schema 产物再生失败（exit ${regen.status ?? 'signal'}）——` +
      '先确认 vbumpp 已构建（cargo build --release -p vbumpp）',
  )
  process.exit(1)
}

// 2. diff 提交产物——漂移即红
const diff = spawnSync('git', ['diff', '--exit-code', '--', ...ARTIFACTS], {
  cwd: root,
  stdio: 'inherit',
})
if (diff.status !== 0) {
  console.error(
    `::error::schema 产物漂移——提交产物与 \`vbumpp schema\` 输出不一致\n` +
      `${ARTIFACTS.join(' 与 ')} 是生成物，手改无效；唯一真相是 vbumpp-core 的配置形状结构体。\n` +
      '本地再生成：cargo build --release -p vbumpp && node scripts/regen-schema.ts，然后提交产物变更。',
  )
  process.exit(1)
}

console.log('schema-drift: 产物与 `vbumpp schema` 输出一致')
