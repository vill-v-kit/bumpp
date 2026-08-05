#!/usr/bin/env node
/**
 * crates.io 上架（COL-54，ADR-0021 决策⑤的消费侧）：对 2 个 crate 按硬约束顺序
 * vbumpp-core → vbumpp（cli 依赖 core）逐一过 publish-guard（COL-51），
 * 未上架的执行 `cargo publish --dry-run` 前置验证后真实上架，已上架的跳过。
 *
 * 用法（ci.yml publish-crates job）：
 *   CARGO_REGISTRY_TOKEN=… node scripts/crates-publish.mjs            # 实际上架
 *   node scripts/crates-publish.mjs --dry-run                         # 干跑（只做前置验证，不上传）
 *
 * 为什么是交错结构（dry-run core → 上架 core → dry-run cli → 上架 cli）而非
 * ticket 字面的「先全部 dry-run 再上架」：首发布蛋——cli 的 dry-run 要把改写后
 * 的 path 依赖（vbumpp-core）向 registry index 解析，core 未真实上架时解析必败
 * （COL-50 已实测）。交错后任一时刻 cli 的 dry-run 都可解析：core 要么刚被本
 * 次运行上架、要么早已在架上（守卫 SKIP 的情形）。
 *
 * 行为契约：
 *   - 版本取自根 Cargo.toml [workspace.package].version（唯一维护点）
 *   - 守卫 GO/SKIP 行透传 stderr；SKIP 跳过该 crate 继续下一个（部分失败重跑
 *     收敛：core SKIP + cli GO 只补发 cli）；守卫 exit 2 → 本脚本 exit 2、
 *     零 cargo 调用
 *   - 每个 GO 的 crate：先 `cargo publish --dry-run -p <name>`，失败则在上传
 *     动作之前整体失败（AC：dry-run 失败不上传）；后续 crate 不再继续（顺序
 *     是硬约束）
 *   - 本次运行一旦有 crate 真实上架成功，后续 crate 的 dry-run 启用重试预算
 *     （默认 10 次 × 15s）——兼做 sparse index 传播延迟的就绪探针；重试参数
 *     可经 CRATES_PUBLISH_RETRY_MAX / CRATES_PUBLISH_RETRY_DELAY_MS 覆盖
 *   - --dry-run 模式：只做上述前置验证（每个 GO 的 crate 各一次 dry-run），
 *     不执行真实上架
 *   - cargo 退出码原样透传；CRATES_PUBLISH_CARGO 可替换 cargo 二进制（测试缝）
 *
 * 认证：cargo 原生读取 CARGO_REGISTRY_TOKEN 环境变量，无需配置文件。
 */
import { execFile } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const guard = fileURLToPath(new URL('./publish-guard.mjs', import.meta.url))
const dryRun = process.argv.includes('--dry-run')
const CARGO = process.env.CRATES_PUBLISH_CARGO ?? 'cargo'
const RETRY_MAX = Number.parseInt(process.env.CRATES_PUBLISH_RETRY_MAX ?? '10', 10)
const RETRY_DELAY_MS = Number.parseInt(process.env.CRATES_PUBLISH_RETRY_DELAY_MS ?? '15000', 10)

// 上架顺序是硬约束：cli 依赖 core（ADR-0021）
const CRATES = ['vbumpp-core', 'vbumpp']

// 版本唯一维护点：根 Cargo.toml [workspace.package].version（ADR-0009 链）
const toml = readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8')
const versionMatch = toml.match(/\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s)
if (!versionMatch) {
  console.error('ERROR cannot read [workspace.package].version from root Cargo.toml')
  process.exit(2)
}
const version = versionMatch[1]

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

// 执行外部命令：exit code 进返回值（不 reject），输出仅捕获不透传——
// 往哪条流 relay 由调用方决定（守卫的信息行进 stderr；cargo 的输出是主体，原样回 stdio）
function run(cmd, args) {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { cwd: root }, (error, stdout, stderr) => {
      if (error && error.code === undefined) return reject(error) // spawn 本身失败
      resolve({ code: error ? error.code : 0, stdout, stderr })
    })
  })
}

let publishedThisRun = false
for (const name of CRATES) {
  // 1. 守卫（GO/SKIP/ERROR 均为信息行 → stderr，保持 stdout 干净）
  const g = await run('node', [guard, 'crates', name, version])
  if (g.stdout) process.stderr.write(g.stdout)
  if (g.stderr) process.stderr.write(g.stderr)
  if (g.code === 2) process.exit(2) // 查询失败：零 cargo 调用，整体失败
  if (g.code === 1) continue // SKIP：已上架，跳过本 crate

  // 2. dry-run 前置（本次运行已上架过 crate → 启用重试预算做 index 传播探针）
  const attempts = publishedThisRun ? RETRY_MAX : 1
  let dr
  for (let i = 1; i <= attempts; i += 1) {
    dr = await run(CARGO, ['publish', '--dry-run', '-p', name])
    if (dr.stdout) process.stdout.write(dr.stdout)
    if (dr.stderr) process.stderr.write(dr.stderr)
    if (dr.code === 0) break
    if (i < attempts) {
      console.error(
        `cargo publish --dry-run -p ${name} attempt ${i}/${attempts} failed ` +
          `(registry index propagation?), retry in ${RETRY_DELAY_MS}ms`,
      )
      await sleep(RETRY_DELAY_MS)
    }
  }
  if (dr.code !== 0) {
    console.error(`ERROR cargo publish --dry-run -p ${name} failed — aborting before any upload`)
    process.exit(typeof dr.code === 'number' ? dr.code : 1)
  }

  // 3. 真实上架（--dry-run 模式跳过）
  if (dryRun) {
    console.log(`[dry-run] would publish ${name}@${version}`)
    continue
  }
  const p = await run(CARGO, ['publish', '-p', name])
  if (p.stdout) process.stdout.write(p.stdout)
  if (p.stderr) process.stderr.write(p.stderr)
  if (p.code !== 0) {
    console.error(`ERROR cargo publish -p ${name} failed`)
    process.exit(typeof p.code === 'number' ? p.code : 1)
  }
  publishedThisRun = true
}
