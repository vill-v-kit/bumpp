/**
 * demo-cast-drift.mjs 的 CLI 契约测试（COL-93）。
 * Seam：CLI 契约本身——用临时 git 仓库搭 fixture（提交一份 cast 产物），
 * DEMO_CAST_CAPTURE_CMD stub 掉真实采集（真实采集要 release 二进制，太重），
 * 分别演「不改文件（一致）/ 改写文件（漂移）/ 直接失败」三种形态。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { execFile, execFileSync } from 'node:child_process'
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./demo-cast-drift.mjs', import.meta.url))
const CAST_PATH = 'website/app/(home)/demo-casts.ts'
const COMMITTED = '// committed cast artifact\n'

function runDrift(cwd, captureCmd) {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script],
      { cwd, env: { ...process.env, DEMO_CAST_CAPTURE_CMD: captureCmd } },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

const git = (cwd, args) =>
  execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@example.com', ...args], { cwd })

describe('demo-cast-drift', () => {
  let dir
  beforeAll(async () => {
    dir = await mkdtemp(join(tmpdir(), 'demo-cast-drift-'))
    git(dir, ['init', '-q', '-b', 'main'])
    await mkdir(join(dir, dirname(CAST_PATH)), { recursive: true })
    await writeFile(join(dir, CAST_PATH), COMMITTED)
    git(dir, ['add', '-A'])
    git(dir, ['-c', 'commit.gpgsign=false', 'commit', '-q', '-m', 'init'])
  })
  afterAll(() => rm(dir, { recursive: true, force: true }))

  it('采集后产物一致 → exit 0', async () => {
    const r = await runDrift(dir, 'true')
    expect(r.code).toBe(0)
  })

  it('采集后产物漂移 → exit 1，提示再生成命令与「生成物手改无效」', async () => {
    const drift = `node -e "require('fs').appendFileSync('${CAST_PATH}','// drift\\n')"`
    const r = await runDrift(dir, drift)
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/pnpm --filter website capture:home-demo-cast/)
    expect(r.stderr).toMatch(/生成物/)
    // 校验只报告，不把漂移内容留在 fixture 里（后续用例还要用同一 fixture）
    git(dir, ['checkout', '--', CAST_PATH])
  })

  it('采集命令本身失败 → exit 1，报采集失败而非漂移，并给出构建与 macOS 依赖提示', async () => {
    const r = await runDrift(dir, 'false')
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/采集失败/)
    expect(r.stderr).toMatch(/cargo build --release -p vbumpp/)
    expect(r.stderr).toMatch(/macOS/)
  })
})
