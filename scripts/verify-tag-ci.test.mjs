/**
 * verify-tag-ci.mjs 的 CLI 契约测试（COL-62）。
 * Seam：CLI 契约本身——spawn 真实脚本进程，打本地 stub GitHub API 验证 exit code。
 * 发版路径唯一要防的事故形态：tag push 事件被 GitHub 丢失 → 零 workflow run →
 * 上架静默不发生；脚本必须在此时 exit 1 并给出删 tag 重推的恢复指引。
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { createServer } from 'node:http'
import { execFile } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const script = fileURLToPath(new URL('./verify-tag-ci.mjs', import.meta.url))

let server
let base
// 每个测试替换此 handler 来模拟 GitHub API 行为
let handler = (_req, res) => res.writeHead(500).end()

beforeAll(async () => {
  server = createServer((req, res) => handler(req, res))
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  base = `http://127.0.0.1:${server.address().port}`
})

afterAll(() => server.close())

function runVerify(args, extraEnv = {}, cwd) {
  return new Promise((resolve) => {
    execFile(
      'node',
      [script, ...args],
      {
        cwd,
        env: {
          ...process.env,
          VERIFY_TAG_CI_API_URL: base,
          VERIFY_TAG_CI_REPO: 'vill-v-kit/bumpp',
          VERIFY_TAG_CI_TIMEOUT_MS: '600',
          VERIFY_TAG_CI_INTERVAL_MS: '100',
          ...extraEnv,
        },
      },
      (error, stdout, stderr) => {
        resolve({ code: error ? error.code : 0, stdout, stderr })
      },
    )
  })
}

function json(res, body) {
  res.writeHead(200, { 'content-type': 'application/json' }).end(JSON.stringify(body))
}

const freshRun = (over = {}) => ({
  id: 31038649040,
  name: 'CI',
  head_branch: 'v9.9.9',
  created_at: new Date().toISOString(),
  ...over,
})

describe('触发核验', () => {
  it('首轮轮询即见 run → exit 0 + OK', async () => {
    handler = (_req, res) => json(res, { workflow_runs: [freshRun()] })
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(0)
    expect(r.stdout).toMatch(/^OK v9\.9\.9 /)
  })

  it('截止前 run 出现（第 3 轮）→ exit 0——轮询不是单次探测', async () => {
    let calls = 0
    handler = (_req, res) => {
      calls += 1
      json(res, { workflow_runs: calls >= 3 ? [freshRun()] : [] })
    }
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(0)
    expect(calls).toBeGreaterThanOrEqual(3)
  })

  it('窗口内仅他 tag 的 run → exit 1（不得张冠李戴）', async () => {
    handler = (_req, res) => json(res, { workflow_runs: [freshRun({ head_branch: 'v8.0.0' })] })
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(1)
  })

  it('仅陈旧 run（created_at 早于窗口）→ exit 1——旧 run 不得判本次已触发', async () => {
    handler = (_req, res) =>
      json(res, {
        workflow_runs: [freshRun({ created_at: new Date(Date.now() - 600_000).toISOString() })],
      })
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(1)
  })

  it('超时仍无 run → exit 1 + LOST + 恢复指引含删 tag 重推命令', async () => {
    handler = (_req, res) => json(res, { workflow_runs: [] })
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(1)
    expect(r.stderr).toMatch(/^LOST v9\.9\.9 /)
    expect(r.stderr).toContain('git push origin :refs/tags/v9.9.9')
    expect(r.stderr).toContain('git push origin v9.9.9')
  })
})

describe('核验失败与「事件丢失」可区分', () => {
  it('API 持续 500 → exit 2 + ERROR（核验不能 ≠ 事件丢失）', async () => {
    handler = (_req, res) => res.writeHead(500).end()
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(2)
    expect(r.stderr).toMatch(/^ERROR /)
  })

  it('先 500 后 200 有 run → exit 0——瞬态故障容忍', async () => {
    let calls = 0
    handler = (_req, res) => {
      calls += 1
      if (calls < 3) return res.writeHead(500).end()
      json(res, { workflow_runs: [freshRun()] })
    }
    const r = await runVerify(['v9.9.9'])
    expect(r.code).toBe(0)
  })
})

function sh(cmd, args, cwd) {
  return new Promise((resolve, reject) =>
    execFile(cmd, args, { cwd }, (e) => (e ? reject(e) : resolve())),
  )
}

/** 临时 git 仓库（可选 origin URL）；调用方负责 finally rmSync */
async function makeGitRepo(remoteUrl) {
  const dir = mkdtempSync(join(tmpdir(), 'verify-tag-ci-'))
  await sh('git', ['init', '-q'], dir)
  if (remoteUrl) await sh('git', ['remote', 'add', 'origin', remoteUrl], dir)
  return dir
}

describe('入参与来源解析', () => {
  it('非 git 目录且无 argv/env 来源 → exit 2（无法解析不得静默通过）', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'verify-tag-ci-'))
    try {
      const r = await runVerify([], { VERIFY_TAG_CI_REPO: '' }, dir)
      expect(r.code).toBe(2)
      expect(r.stderr).toMatch(/^ERROR /)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it.each([
    ['git@github.com:foo/bar.git', 'ssh 形态'],
    ['https://github.com/foo/bar', 'https 形态'],
  ])('origin %s（%s）→ 解析 owner/repo 并命中对应 API 路径', async (url) => {
    const dir = await makeGitRepo(url)
    try {
      let seenUrl
      handler = (req, res) => {
        seenUrl = req.url
        json(res, { workflow_runs: [freshRun()] })
      }
      const r = await runVerify(['v9.9.9'], { VERIFY_TAG_CI_REPO: '' }, dir)
      expect(r.code).toBe(0)
      expect(seenUrl).toMatch(/^\/repos\/foo\/bar\/actions\/runs/)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it('无 argv 时回落 `git describe --tags --abbrev=0` 取最近 tag', async () => {
    const dir = await makeGitRepo('git@github.com:foo/bar.git')
    try {
      await sh('git', ['-c', 'user.email=t@t', '-c', 'user.name=t', 'commit', '-q', '--allow-empty', '-m', 'init'], dir)
      await sh('git', ['tag', 'v1.2.3'], dir)
      handler = (_req, res) => json(res, { workflow_runs: [freshRun({ head_branch: 'v1.2.3' })] })
      const r = await runVerify([], { VERIFY_TAG_CI_REPO: '' }, dir)
      expect(r.code).toBe(0)
      expect(r.stdout).toMatch(/^OK v1\.2\.3 /)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })
})
