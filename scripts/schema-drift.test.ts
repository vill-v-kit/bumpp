/**
 * schema-drift.ts 的 CLI 契约测试。
 * Seam：CLI 契约本身——用临时 git 仓库搭 fixture（提交两处 schema 产物），
 * SCHEMA_REGEN_CMD stub 掉真实再生（真实再生要 vbumpp 二进制，太重），
 * 分别演「不改文件（一致）/ 改写文件（漂移）/ 直接失败」三种形态。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { execFile, execFileSync } from 'node:child_process'
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./schema-drift.ts', import.meta.url))
const ARTIFACTS = ['npm/bump/vbumpprc.schema.json', 'website/public/vbumpprc.schema.json']
const COMMITTED = '{ "type": "object" }\n'

interface RunResult {
  code: string | number | undefined
  stdout: string
  stderr: string
}

function runDrift(cwd: string, regenCmd: string): Promise<RunResult> {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script],
      { cwd, env: { ...process.env, SCHEMA_REGEN_CMD: regenCmd } },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

const git = (cwd: string, args: string[]) =>
  execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.com', ...args], { cwd })

describe('schema-drift', () => {
  let dir: string
  beforeAll(async () => {
    dir = await mkdtemp(join(tmpdir(), 'schema-drift-'))
    git(dir, ['init', '-q', '-b', 'main'])
    for (const rel of ARTIFACTS) {
      await mkdir(join(dir, dirname(rel)), { recursive: true })
      await writeFile(join(dir, rel), COMMITTED)
    }
    git(dir, ['add', '-A'])
    git(dir, ['-c', 'commit.gpgsign=false', 'commit', '-q', '-m', 'init'])
  })
  afterAll(() => rm(dir, { recursive: true, force: true }))

  it('再生后产物一致 → exit 0', async () => {
    const r = await runDrift(dir, 'true')
    expect(r.code).toBe(0)
  })

  it('再生后产物漂移 → exit 1，提示再生成命令与「生成物手改无效」', async () => {
    const drift = `node -e "require('fs').appendFileSync('${ARTIFACTS[0]}','// drift\\n')"`
    const r = await runDrift(dir, drift)
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/node scripts\/regen-schema\.ts/)
    expect(r.stderr).toMatch(/生成物/)
    // 校验只报告，不把漂移内容留在 fixture 里（后续用例还要用同一 fixture）
    git(dir, ['checkout', '--', ...ARTIFACTS])
  })

  it('再生命令本身失败 → exit 1，报再生失败而非漂移，并给出构建提示', async () => {
    const r = await runDrift(dir, 'false')
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/再生失败/)
    expect(r.stderr).toMatch(/cargo build --release -p vbumpp/)
  })
})
