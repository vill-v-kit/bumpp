/**
 * 发版 tag CI 触发核验：v6.0.0 实例——tag push 的 ref 更新到达 GitHub，
 * 但其下游事件被丢失，两个 workflow 零 run、零 check-suite，上架静默不发生；
 * 删 tag 重推即恢复。本脚本在 tag 推送后轮询 Actions runs，把「事件丢失」
 * 从事后人工察觉（本实例滞后 8 分钟）变成发版当场告警。
 *
 * 用法：node scripts/verify-tag-ci.ts [tag]
 *   tag 缺省取 `git describe --tags --abbrev=0`（最近一次 tag）
 *
 * 退出码契约：
 *   0 + stdout OK …   —— 窗口内出现该 tag 的 push run，触发无恙
 *   1 + stderr LOST … —— 轮询至超时仍无 run：push 事件疑似被 GitHub 丢失，
 *                        输出删 tag 重推的恢复指引
 *   2 + stderr ERROR …—— 核验本身失败（入参/来源解析失败、API 始终不可达），
 *                        与「事件丢失」严格区分，消费方必须视为失败
 *
 * 只读查询：公开仓库无需凭证；GITHUB_TOKEN / GH_TOKEN 存在则携带（私有仓库
 * 或避开匿名 rate limit）。时序基准：run 的 created_at 须落在
 * 「脚本启动 − skew」之后（默认 120s），防止把上一次发版的旧 run 误判为
 * 本次已触发——接入 `pnpm release` 时脚本紧随 push 执行，窗口天然精确。
 *
 * 环境变量覆盖（自建 GitHub / 测试 stub 通用）：
 *   VERIFY_TAG_CI_API_URL     （默认 https://api.github.com）
 *   VERIFY_TAG_CI_REPO        （owner/repo，缺省解析 `git remote get-url origin`）
 *   VERIFY_TAG_CI_TIMEOUT_MS  （默认 180000）
 *   VERIFY_TAG_CI_INTERVAL_MS （默认 10000）
 *   VERIFY_TAG_CI_SKEW_MS     （默认 120000）
 */
import { execFile } from 'node:child_process'

const GUARD_UA = 'vill-v-kit/bumpp verify-tag-ci (https://github.com/vill-v-kit/bumpp)'

const timeoutMs = Number(process.env.VERIFY_TAG_CI_TIMEOUT_MS ?? 180_000)
const intervalMs = Number(process.env.VERIFY_TAG_CI_INTERVAL_MS ?? 10_000)
const skewMs = Number(process.env.VERIFY_TAG_CI_SKEW_MS ?? 120_000)

function fail(message: string): never {
  console.error(`ERROR ${message}`)
  process.exit(2)
}

function git(args: string[]): Promise<string | null> {
  return new Promise((resolve) => {
    execFile('git', args, { encoding: 'utf8' }, (error, stdout) => {
      resolve(error ? null : stdout.trim())
    })
  })
}

/** origin URL → owner/repo；ssh（git@github.com:o/r.git）与 https 两形态，其余 null */
function parseGithubRemote(url: string | null): string | null {
  const m = /github\.com[:/](?<repo>[^/]+\/[^/]+?)(?:\.git)?$/.exec(url ?? '')
  return m?.groups?.repo ?? null
}

const [tagArg] = process.argv.slice(2)
const tag = tagArg || (await git(['describe', '--tags', '--abbrev=0']))
if (!tag) {
  fail('no tag given and `git describe --tags --abbrev=0` failed — pass the tag explicitly')
}

const repo =
  process.env.VERIFY_TAG_CI_REPO || parseGithubRemote(await git(['remote', 'get-url', 'origin']))
if (!repo) {
  fail('cannot resolve owner/repo from `git remote get-url origin` (expected a github.com remote)')
}

const apiBase = (process.env.VERIFY_TAG_CI_API_URL ?? 'https://api.github.com').replace(/\/$/, '')
const url = `${apiBase}/repos/${repo}/actions/runs?event=push&per_page=30`

const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN
const headers: Record<string, string> = { 'user-agent': GUARD_UA }
if (token) headers.authorization = `Bearer ${token}`

const startedMs = Date.now()
const floorMs = startedMs - skewMs
const deadlineMs = startedMs + timeoutMs

let everOk = false
let lastError = 'no response'
for (;;) {
  try {
    const res = await fetch(url, { headers })
    if (res.ok) {
      everOk = true
      const body = (await res.json()) as { workflow_runs?: { head_branch: string; created_at: string; name: string; id: number }[] }
      const runs = (body.workflow_runs ?? []).filter(
        (run) => run.head_branch === tag && Date.parse(run.created_at) >= floorMs,
      )
      if (runs.length > 0) {
        const names = runs.map((run) => `${run.name}#${run.id}`).join(', ')
        console.log(`OK ${tag} — ${runs.length} workflow run(s) created (${names})`)
        process.exit(0)
      }
    } else {
      lastError = `HTTP ${res.status}`
    }
  } catch (err) {
    const e = err as Error & { cause?: { code?: string } }
    lastError = e.cause?.code ?? e.message
  }

  if (Date.now() >= deadlineMs) break
  await new Promise((resolve) => setTimeout(resolve, intervalMs))
}

if (!everOk) {
  fail(`cannot verify CI trigger for ${tag} — GitHub API unreachable throughout (${lastError}); check https://github.com/${repo}/actions manually`)
}

console.error(
  `LOST ${tag} — no workflow run within ${Math.round(timeoutMs / 1000)}s; the tag push event was likely dropped by GitHub`,
)
console.error('')
console.error('recover by deleting and re-pushing the tag (same object, forces a fresh push event):')
console.error(`  git push origin :refs/tags/${tag}`)
console.error(`  git push origin ${tag}`)
process.exit(1)
