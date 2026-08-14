/**
 * npm-publish.ts 的 CLI 契约测试（COL-53）。
 * Seam：CLI 契约本身——spawn 真实脚本进程（--dry-run 不上传），经
 * PUBLISH_GUARD_NPM_URL 把 publish-guard 查询与 pnpm publish 的 registry
 * 检查都导向本地 stub，验证「守卫过滤 → 放行上架」全链路的构成。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http'
import type { AddressInfo } from 'node:net'
import { execFile } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./npm-publish.ts', import.meta.url))

// 当前仓库的 13 个可上架包（website 与根 monorepo 均 private，永不出现在放行集）；
// 新增发版包时此清单需同步——恰好全覆盖是 COL-53 的验收项
const PUBLISHABLE = [
  '@vill-v/bumpp-core',
  '@vill-v/bumpp-core-darwin-arm64',
  '@vill-v/bumpp-core-linux-arm64-gnu',
  '@vill-v/bumpp-core-linux-arm64-musl',
  '@vill-v/bumpp-core-linux-x64-gnu',
  '@vill-v/bumpp-core-linux-x64-musl',
  '@vill-v/bumpp-core-win32-arm64-msvc',
  '@vill-v/bumpp-core-win32-x64-msvc',
  '@vill-v/bumpp',
  '@vill-v/bumpp-gitcode',
  '@vill-v/bumpp-gitee',
  '@vill-v/bumpp-github',
  '@vill-v/bumpp-gitlab',
]

type Handler = (req: IncomingMessage, res: ServerResponse) => void

interface RunResult {
  code: string | number | undefined
  stdout: string
  stderr: string
}

let server: Server
let base: string
// 每个测试替换此 handler 来模拟 registry 行为
let handler: Handler = (_req, res) => res.writeHead(500).end()

const root = fileURLToPath(new URL('..', import.meta.url))
const exec = (cmd: string, args: string[]): Promise<string> =>
  new Promise((resolve, reject) => {
    execFile(cmd, args, { cwd: root }, (error, stdout, stderr) =>
      error
        ? reject(new Error(`${cmd} ${args.join(' ')} failed: ${stderr || error.message}`))
        : resolve(stdout),
    )
  })

beforeAll(async () => {
  // 平台包目录不提交进 git（ADR-0029）：枚举前先生成并链接，否则 pnpm ls -r
  // 看不到 7 个平台包、publish --dry-run 也无法完成 workspace:* 版本改写；
  // frozen 保证测试永不把 lockfile 改写当副作用（提交的 lockfile 已含平台包记录）
  await exec('node', ['scripts/create-npm-dirs.ts'])
  await exec('pnpm', ['install', '--frozen-lockfile'])
  server = createServer((req, res) => handler(req, res))
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
}, 120_000)

afterAll(() => server.close())

function runPublish({ dryRun = true, env = {} }: { dryRun?: boolean; env?: Record<string, string> } = {}): Promise<RunResult> {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script, ...(dryRun ? ['--dry-run'] : [])],
      // CI 显式置空：本测试自身就在 CI 的 test job 里跑，环境传染必须隔离，
      // 各用例经 env 显式声明自己的 CI 状态
      { env: { ...process.env, CI: '', ...env, PUBLISH_GUARD_NPM_URL: base } },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

// pnpm publish --dry-run 输出的「📦 <name>@<version> → …」提取为包名集合
function dryRunPackages(stdout: string) {
  return [...stdout.matchAll(/📦 (\S+?)@/g)].map((m) => m[1]).sort()
}

describe('守卫过滤 → 放行上架', () => {
  it('全部未上架（404）→ 恰好 13 个包进入 publish', { timeout: 60000 }, async () => {
    handler = (_req, res) => res.writeHead(404).end()
    const r = await runPublish()
    expect(r.code).toBe(0)
    expect(dryRunPackages(r.stdout)).toEqual([...PUBLISHABLE].sort())
  })

  it('部分已上架 → publish 只含未上架的，SKIP 行进 stderr', { timeout: 60000 }, async () => {
    const published = ['@vill-v/bumpp', '@vill-v/bumpp-core-darwin-arm64']
    handler = (req, res) => {
      const name = decodeURIComponent((req.url ?? '').split('/')[1])
      res.writeHead(published.includes(name) ? 200 : 404).end()
    }
    const r = await runPublish()
    expect(r.code).toBe(0)
    expect(dryRunPackages(r.stdout)).toEqual(PUBLISHABLE.filter((n) => !published.includes(n)).sort())
    for (const p of published) {
      expect(r.stderr).toContain(`SKIP ${p}@`)
    }
  })

  it('全部已上架（200）→ 不调用 pnpm publish 直接收工', { timeout: 60000 }, async () => {
    handler = (_req, res) => res.writeHead(200).end('{}')
    const r = await runPublish()
    expect(r.code).toBe(0)
    expect(r.stdout).not.toContain('📦')
    expect(r.stdout + r.stderr).toMatch(/nothing to do|全部已上架/)
  })
})

describe('查询失败原子性', () => {
  it('任一包查询失败（500）→ exit 2 且不发起 publish', { timeout: 60000 }, async () => {
    handler = (req, res) => {
      const name = decodeURIComponent((req.url ?? '').split('/')[1])
      res.writeHead(name === '@vill-v/bumpp-gitlab' ? 500 : 404).end()
    }
    const r = await runPublish()
    expect(r.code).toBe(2)
    expect(r.stdout).not.toContain('📦')
    expect(r.stderr).toContain('ERROR')
  })
})

describe('全新包名前置检测（首发仪式触发绑定）', () => {
  // 守卫的 per-version 查询与前置检测的包级探针（/<name>/latest）同打到
  // stub：404 = 未上架。全部 404 时 13 个包都是「全新包名」
  const allNew: Handler = (_req, res) => res.writeHead(404).end()
  // 包名已存在（包级探针 200）仅版本新（per-version 404 → GO）
  const nameExists: Handler = (req, res) => res.writeHead((req.url ?? '').endsWith('/latest') ? 200 : 404).end()

  it('CI + 全新包名 → exit 2 整体拦停、不发起 publish、指引含首发仪式', { timeout: 60000 }, async () => {
    handler = allNew
    const r = await runPublish({ dryRun: false, env: { CI: '1' } })
    expect(r.code).toBe(2)
    expect(r.stdout).not.toContain('run: pnpm')
    expect(r.stderr).toContain('ERROR 发布计划含从未上架的全新包名')
    expect(r.stderr).toContain('首发仪式')
  })

  it('CI + 包名已存在仅版本新 → 不拦，照常放行 publish', { timeout: 120000 }, async () => {
    handler = nameExists
    const r = await runPublish({ dryRun: false, env: { CI: '1' } })
    expect(r.stderr).not.toContain('全新包名')
    // publish 实际打到 stub registry 必然失败——本例只断言未被前置检测拦停
    expect(r.stdout).toContain('run: pnpm')
  })

  it('非 CI + 全新包名 → 只警告不拦（本地手动首发仪式通路）', { timeout: 120000 }, async () => {
    handler = allNew
    const r = await runPublish({ dryRun: false })
    expect(r.stderr).toContain('WARN 发布计划含从未上架的全新包名')
    expect(r.stdout).toContain('run: pnpm')
  })

  it('dry-run 永不拦：CI 下全新包名也只警告、照常干跑', { timeout: 60000 }, async () => {
    handler = allNew
    const r = await runPublish({ env: { CI: '1' } })
    expect(r.code).toBe(0)
    expect(dryRunPackages(r.stdout)).toEqual([...PUBLISHABLE].sort())
    expect(r.stderr).toContain('WARN 发布计划含从未上架的全新包名')
  })
})
