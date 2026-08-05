/**
 * publish-guard.mjs 的 CLI 契约测试（COL-51）。
 * Seam：CLI 契约本身——spawn 真实脚本进程，打本地 stub registry 验证 exit code。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { createServer } from 'node:http'
import { execFile } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./publish-guard.mjs', import.meta.url))

let server
let base
// 每个测试替换此 handler 来模拟 registry 行为
let handler = (_req, res) => res.writeHead(500).end()

beforeAll(async () => {
  server = createServer((req, res) => handler(req, res))
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  base = `http://127.0.0.1:${server.address().port}`
})

afterAll(() => server.close())

function runGuard(args) {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script, ...args],
      { env: { ...process.env, PUBLISH_GUARD_NPM_URL: base, PUBLISH_GUARD_CRATES_URL: base } },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

describe('crates registry', () => {
  it('404（未上架）→ exit 0 放行', async () => {
    handler = (_req, res) => res.writeHead(404).end()
    const r = await runGuard(['crates', 'vbumpp-core', '6.0.0'])
    expect(r.code).toBe(0)
    expect(r.stdout).toMatch(/^GO /)
  })

  it('200（已上架）→ exit 1 跳过', async () => {
    handler = (_req, res) => res.writeHead(200).end('{}')
    const r = await runGuard(['crates', 'vbumpp', '6.0.0'])
    expect(r.code).toBe(1)
    expect(r.stdout).toMatch(/^SKIP /)
  })
})

describe('npm registry', () => {
  it('404（未上架）→ exit 0 放行', async () => {
    handler = (_req, res) => res.writeHead(404).end()
    const r = await runGuard(['npm', '@vill-v/bumpp', '6.0.0'])
    expect(r.code).toBe(0)
    expect(r.stdout).toMatch(/^GO /)
  })

  it('200（已上架）→ exit 1 跳过', async () => {
    handler = (_req, res) => res.writeHead(200).end('{}')
    const r = await runGuard(['npm', '@vill-v/bumpp', '6.0.0'])
    expect(r.code).toBe(1)
    expect(r.stdout).toMatch(/^SKIP /)
  })

  it('scoped 包名 URL 编码（@scope/name → %40scope%2Fname）', async () => {
    let seenUrl
    handler = (req, res) => {
      seenUrl = req.url
      res.writeHead(404).end()
    }
    await runGuard(['npm', '@vill-v/bumpp-core', '6.0.0'])
    expect(seenUrl).toBe('/%40vill-v%2Fbumpp-core/6.0.0')
  })
})

describe('查询失败与「包不存在」可区分', () => {
  it('500（registry 故障）→ exit 2 而非放行', async () => {
    handler = (_req, res) => res.writeHead(500).end()
    const r = await runGuard(['crates', 'vbumpp-core', '6.0.0'])
    expect(r.code).toBe(2)
    expect(r.stderr).toMatch(/^ERROR /)
  })

  it('网络不可达 → exit 2（fetch 抛错不得穿透成其他退出码）', async () => {
    const r = await new Promise((resolve) => {
      execFile(
        'node',
        [script, 'npm', '@vill-v/bumpp', '6.0.0'],
        // 1 端口必然连不上
        { env: { ...process.env, PUBLISH_GUARD_NPM_URL: 'http://127.0.0.1:1' } },
        (error, stdout, stderr) => resolve({ code: error ? error.code : 0, stdout, stderr }),
      )
    })
    expect(r.code).toBe(2)
    expect(r.stderr).toMatch(/^ERROR /)
  })
})

describe('非法输入', () => {
  it('缺参数 → exit 2 + usage', async () => {
    const r = await runGuard(['crates', 'vbumpp-core'])
    expect(r.code).toBe(2)
    expect(r.stderr).toMatch(/^usage: /)
  })

  it('未知 registry → exit 2（typo 不得静默落到默认通路）', async () => {
    const r = await runGuard(['crates-io', 'vbumpp-core', '6.0.0'])
    expect(r.code).toBe(2)
    expect(r.stderr).toMatch(/^unknown registry: /)
  })
})
