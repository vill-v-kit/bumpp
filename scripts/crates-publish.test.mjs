/**
 * crates-publish.mjs 的 CLI 契约测试（COL-54）。
 * Seam：CLI 契约本身——spawn 真实脚本进程，经 PUBLISH_GUARD_CRATES_URL 把
 * publish-guard 查询导向本地 stub registry，经 CRATES_PUBLISH_CARGO 把 cargo
 * 调用导向 stub 二进制（记 argv 日志、可按注入规则失败），验证
 * 「守卫 → dry-run 前置 → core→cli 顺序 → 重试/跳过」全链路编排。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { createServer } from 'node:http'
import { execFile } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { mkdtempSync, writeFileSync, readFileSync, chmodSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const script = fileURLToPath(new URL('./crates-publish.mjs', import.meta.url))

let server
let base
// 每个测试替换此 handler 来模拟 crates.io API 行为
let handler = (_req, res) => res.writeHead(500).end()

beforeAll(async () => {
  server = createServer((req, res) => handler(req, res))
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  base = `http://127.0.0.1:${server.address().port}`
})

afterAll(() => server.close())

// ---- stub cargo：记录 argv 到 CARGO_LOG；按 CARGO_FAKE_FAIL（JSON：{命令串: 剩余失败次数}）失败
const FAKE_CARGO = `#!/usr/bin/env node
import { appendFileSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
const key = process.argv.slice(2).join(' ')
appendFileSync(process.env.CARGO_LOG, key + '\\n')
const failRules = JSON.parse(process.env.CARGO_FAKE_FAIL ?? '{}')
const statePath = process.env.CARGO_STATE
const state = existsSync(statePath) ? JSON.parse(readFileSync(statePath, 'utf8')) : {}
const used = state[key] ?? 0
const remaining = failRules[key] ?? 0
if (used < remaining) {
  state[key] = used + 1
  writeFileSync(statePath, JSON.stringify(state))
  console.error('fake cargo: simulated failure for ' + key)
  process.exit(101)
}
console.log('fake cargo ok: ' + key)
`

function makeFakeCargo(failRules = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'crates-publish-test-'))
  const bin = join(dir, 'fake-cargo.mjs')
  writeFileSync(bin, FAKE_CARGO)
  chmodSync(bin, 0o755)
  return {
    bin,
    env: {
      CRATES_PUBLISH_CARGO: bin,
      CARGO_LOG: join(dir, 'cargo.log'),
      CARGO_STATE: join(dir, 'cargo-state.json'),
      CARGO_FAKE_FAIL: JSON.stringify(failRules),
    },
    logPath: join(dir, 'cargo.log'),
  }
}

function readLog(fake) {
  try {
    return readFileSync(fake.logPath, 'utf8').trim().split('\n')
  } catch {
    return []
  }
}

function runPublish(fake, args = []) {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script, ...args],
      {
        env: {
          ...process.env,
          ...fake.env,
          PUBLISH_GUARD_CRATES_URL: base,
          CRATES_PUBLISH_RETRY_DELAY_MS: '10',
        },
      },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

const statusByCrate = (statuses) => (req, res) => {
  // /api/v1/crates/<name>/<version> → ['', 'api', 'v1', 'crates', <name>, <version>]
  const name = decodeURIComponent(req.url.split('/')[4])
  res.writeHead(statuses[name] ?? 404).end()
}

describe('守卫 → dry-run 前置 → core→cli 顺序', () => {
  it('双双未上架 + --dry-run：两 crate 各一次 dry-run，无真实上架', { timeout: 30000 }, async () => {
    handler = statusByCrate({})
    const fake = makeFakeCargo()
    const r = await runPublish(fake, ['--dry-run'])
    expect(r.code).toBe(0)
    expect(readLog(fake)).toEqual([
      'publish --dry-run -p vbumpp-core',
      'publish --dry-run -p vbumpp',
    ])
  })

  it('双双未上架（真跑）：core→cli 顺序，各自 dry-run 先行', { timeout: 30000 }, async () => {
    handler = statusByCrate({})
    const fake = makeFakeCargo()
    const r = await runPublish(fake)
    expect(r.code).toBe(0)
    expect(readLog(fake)).toEqual([
      'publish --dry-run -p vbumpp-core',
      'publish -p vbumpp-core',
      'publish --dry-run -p vbumpp',
      'publish -p vbumpp',
    ])
  })

  it('core 已上架、cli 未上架（部分失败重跑收敛）：只补发 cli', { timeout: 30000 }, async () => {
    handler = statusByCrate({ 'vbumpp-core': 200 })
    const fake = makeFakeCargo()
    const r = await runPublish(fake)
    expect(r.code).toBe(0)
    expect(readLog(fake)).toEqual([
      'publish --dry-run -p vbumpp',
      'publish -p vbumpp',
    ])
    expect(r.stderr).toContain('SKIP vbumpp-core@')
  })

  it('双双已上架（全绿重跑）：零 cargo 调用直接收工', { timeout: 30000 }, async () => {
    handler = statusByCrate({ 'vbumpp-core': 200, vbumpp: 200 })
    const fake = makeFakeCargo()
    const r = await runPublish(fake)
    expect(r.code).toBe(0)
    expect(readLog(fake)).toEqual([])
  })
})

describe('失败语义', () => {
  it('core 的 dry-run 失败 → 在上传动作之前失败，core 真实上架零调用', { timeout: 30000 }, async () => {
    handler = statusByCrate({})
    const fake = makeFakeCargo({ 'publish --dry-run -p vbumpp-core': Number.MAX_SAFE_INTEGER })
    const r = await runPublish(fake)
    expect(r.code).not.toBe(0)
    const log = readLog(fake)
    expect(log).not.toContain('publish -p vbumpp-core')
    expect(log).not.toContain('publish -p vbumpp')
    // core 失败后 cli 不得继续（顺序是硬约束）
    expect(log.every((l) => !l.includes('-p vbumpp ') && !l.endsWith('-p vbumpp'))).toBe(true)
  })

  it('index 传播延迟：core 上架后 cli 的 dry-run 前两次失败 → 重试后成功', { timeout: 30000 }, async () => {
    handler = statusByCrate({})
    const fake = makeFakeCargo({ 'publish --dry-run -p vbumpp': 2 })
    const r = await runPublish(fake)
    expect(r.code).toBe(0)
    const log = readLog(fake)
    expect(log.filter((l) => l === 'publish --dry-run -p vbumpp')).toHaveLength(3)
    expect(log.at(-1)).toBe('publish -p vbumpp')
  })

  it('任一包守卫查询失败（500）→ exit 2 且零 cargo 调用', { timeout: 30000 }, async () => {
    handler = statusByCrate({ 'vbumpp-core': 500 })
    const fake = makeFakeCargo()
    const r = await runPublish(fake)
    expect(r.code).toBe(2)
    expect(readLog(fake)).toEqual([])
    expect(r.stderr).toContain('ERROR')
  })
})
