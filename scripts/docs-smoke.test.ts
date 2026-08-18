/**
 * docs-smoke.ts 的 CLI 契约测试。
 * Seam：CLI 契约本身——assert-artifacts 用临时目录搭产物 fixture，
 * check-live 打本地 stub 站点验证轮询与退出码。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http'
import type { AddressInfo } from 'node:net'
import { execFile } from 'node:child_process'
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./docs-smoke.ts', import.meta.url))
const SITE = 'https://vill-v-kit.github.io/bumpp'

interface RunResult {
  code: string | number | undefined
  stdout: string
  stderr: string
}

function runSmoke(args: string[], env: Record<string, string> = {}): Promise<RunResult> {
  return new Promise((resolve) => {
    execFile('node', [script, ...args], { env: { ...process.env, ...env } }, (error, stdout, stderr) => {
      resolve({ code: error ? error.code : 0, stdout, stderr })
    })
  })
}

/** 搭一个「合格」的产物 fixture，各用例在其上删改出反例 */
async function makeFixture() {
  const dir = await mkdtemp(join(tmpdir(), 'docs-smoke-'))
  await mkdir(join(dir, 'api'), { recursive: true })
  await mkdir(join(dir, '_next/static/chunks'), { recursive: true })
  await writeFile(join(dir, 'api', 'search'), '{"type":"advanced"}')
  // 库代码内嵌的默认形参（合法存在）+ 我们显式传入的带 basePath 地址
  await writeFile(
    join(dir, '_next/static/chunks/app.js'),
    'a="/api/search";b="/bumpp/api/search"',
  )
  await writeFile(
    join(dir, 'llms.txt'),
    [
      '- [文档](https://vill-v-kit.github.io/bumpp/docs)',
      '- [迁移](https://vill-v-kit.github.io/bumpp/docs/migration-v6)',
      '- [发布说明](https://github.com/vill-v-kit/bumpp/releases)', // 异源链接不受约束
    ].join('\n'),
  )
  // schema 产物随静态导出
  await writeFile(join(dir, 'vbumpprc.schema.json'), '{"type":"object"}')
  return dir
}

describe('assert-artifacts', () => {
  let dir: string
  beforeAll(async () => {
    dir = await makeFixture()
  })
  afterAll(() => rm(dir, { recursive: true, force: true }))

  it('合格产物 → exit 0', async () => {
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(0)
  })

  it('搜索索引缺失 → exit 1', async () => {
    await rm(join(dir, 'api', 'search'))
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/搜索索引未导出/)
    await writeFile(join(dir, 'api', 'search'), '{"type":"advanced"}')
  })

  it('bundle 只剩无前缀 /api/search（回归形态）→ exit 1', async () => {
    await writeFile(join(dir, '_next/static/chunks/app.js'), 'a="/api/search"')
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/丢了 basePath/)
    await writeFile(join(dir, '_next/static/chunks/app.js'), 'b="/bumpp/api/search"')
  })

  it('bundle 为模板字面量形态（运行时 basePath 拼接）→ exit 0', async () => {
    await writeFile(
      join(dir, '_next/static/chunks/app.js'),
      'from:`${l.basePath}/api/search`',
    )
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(0)
    await writeFile(join(dir, '_next/static/chunks/app.js'), 'b="/bumpp/api/search"')
  })

  it('llms.txt 同源链接缺 basePath → exit 1', async () => {
    await writeFile(join(dir, 'llms.txt'), '- [文档](https://vill-v-kit.github.io/docs)')
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/同源链接未带 basePath/)
    await writeFile(join(dir, 'llms.txt'), '- [文档](https://vill-v-kit.github.io/bumpp/docs)')
  })

  it('schema 产物缺失 → exit 1', async () => {
    await rm(join(dir, 'vbumpprc.schema.json'))
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/schema 产物未进静态导出/)
    await writeFile(join(dir, 'vbumpprc.schema.json'), '{"type":"object"}')
  })

  it('schema 产物损坏（非 JSON）→ exit 1', async () => {
    await writeFile(join(dir, 'vbumpprc.schema.json'), 'not json')
    const r = await runSmoke(['assert-artifacts', dir, SITE])
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/不是合法 JSON/)
    await writeFile(join(dir, 'vbumpprc.schema.json'), '{"type":"object"}')
  })

  it('参数缺失 → exit 2', async () => {
    const r = await runSmoke(['assert-artifacts', dir])
    expect(r.code).toBe(2)
  })
})

describe('check-live', () => {
  let server: Server
  let base: string
  let handler: (req: IncomingMessage, res: ServerResponse) => void = (_req, res) => res.writeHead(200).end('ok')

  beforeAll(async () => {
    server = createServer((req, res) => handler(req, res))
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
  })
  afterAll(() => server.close())

  const fastEnv = { DOCS_SMOKE_ROUNDS: '4', DOCS_SMOKE_INTERVAL_MS: '10' }

  it('全部 200 → exit 0', async () => {
    handler = (_req, res) => res.writeHead(200).end('ok')
    const r = await runSmoke(['check-live', base], fastEnv)
    expect(r.code).toBe(0)
  })

  it('先 404 后 200（模拟 Pages 传播延迟）→ exit 0', async () => {
    // 4 个资源（/、/api/search、/llms.txt、/vbumpprc.schema.json）：第一轮全 404、第二轮起全 200
    let hits = 0
    handler = (_req, res) => {
      hits += 1
      res.writeHead(hits > 4 ? 200 : 404).end()
    }
    const r = await runSmoke(['check-live', base], fastEnv)
    expect(r.code).toBe(0)
  })

  it('持续 404 → 轮询超时 exit 1', async () => {
    handler = (_req, res) => res.writeHead(404).end()
    const r = await runSmoke(['check-live', base], fastEnv)
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/仍未 200/)
  })
})
