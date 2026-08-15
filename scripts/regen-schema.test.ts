/**
 * regen-schema.ts 的 CLI 契约测试（COL-104）。
 * Seam：二进制解析与落盘契约——临时 git 仓库搭 fixture，VBUMPP_BIN 指向 stub
 * 可执行（shebang node 脚本吐固定内容），不依赖真实 cargo 构建。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { execFile, execFileSync } from 'node:child_process'
import { mkdtemp, writeFile, readFile, chmod, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./regen-schema.ts', import.meta.url))
const ARTIFACTS = ['npm/bump/vbumpprc.schema.json', 'website/public/vbumpprc.schema.json']
const SCHEMA_JSON = '{"type":"object","properties":{}}'

interface RunResult {
  code: string | number | undefined
  stdout: string
  stderr: string
}

function runRegen(cwd: string, bin: string): Promise<RunResult> {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script],
      { cwd, env: { ...process.env, VBUMPP_BIN: bin } },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

/** 写一个 shebang node 的 stub 可执行，运行即打印给定内容 */
async function makeStub(dir: string, name: string, body: string): Promise<string> {
  const path = join(dir, name)
  await writeFile(path, `#!/usr/bin/env node\n${body}\n`)
  await chmod(path, 0o755)
  return path
}

const git = (cwd: string, args: string[]) =>
  execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.com', ...args], { cwd })

describe('regen-schema', () => {
  let dir: string
  beforeAll(async () => {
    dir = await mkdtemp(join(tmpdir(), 'regen-schema-'))
    git(dir, ['init', '-q', '-b', 'main'])
  })
  afterAll(() => rm(dir, { recursive: true, force: true }))

  it('stub 吐纯 JSON → exit 0，两处产物落盘且尾换行归一', async () => {
    // stub 刻意不打尾换行——落盘内容必须恰好一个尾换行
    const stub = await makeStub(dir, 'vbumpp-stub', `process.stdout.write('${SCHEMA_JSON}')`)
    const r = await runRegen(dir, stub)
    expect(r.code).toBe(0)
    for (const rel of ARTIFACTS) {
      expect(await readFile(join(dir, rel), 'utf8')).toBe(`${SCHEMA_JSON}\n`)
    }
  })

  it('stub 输出非纯 JSON → exit 1，拒绝落盘并报错', async () => {
    const stub = await makeStub(dir, 'vbumpp-noisy', "console.log('not json')")
    const r = await runRegen(dir, stub)
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/纯 JSON/)
  })

  it('stub 非零退出 → exit 1，报调用失败', async () => {
    const stub = await makeStub(dir, 'vbumpp-fail', 'process.exit(3)')
    const r = await runRegen(dir, stub)
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/调用失败/)
  })

  it('二进制缺失（VBUMPP_BIN 空且无 target/）→ exit 1，提示构建命令', async () => {
    const r = await runRegen(dir, '')
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/cargo build --release -p vbumpp/)
  })
})
