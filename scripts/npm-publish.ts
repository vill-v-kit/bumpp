/**
 * npm 上架（COL-53，ADR-0021 决策⑤的消费侧）：枚举 workspace 全部非 private 包
 * （当前恰好 13 个——website 与根 monorepo 为 private 自动排除；7 个平台包目录
 * 由 create-npm-dirs 生成、不提交进 git，ADR-0029），逐一过
 * publish-guard（COL-51）查询 registry，只对未上架的包执行 `pnpm publish`。
 * 已上架包跳过、查询失败整体不放行——「Re-run failed jobs」重跑即收敛。
 *
 * 用法（ci.yml publish-npm job）：
 *   NODE_AUTH_TOKEN=… node scripts/npm-publish.ts            # 实际上架
 *   node scripts/npm-publish.ts --dry-run                    # 干跑（本地/CI 验证，不上传）
 *
 * 为什么由脚本而非 yaml 内联 bash 调 pnpm publish：--filter 列表若经 shell 变量
 * 传递依赖单词拆分，bash 拆、zsh 不拆（本地复跑 yaml 块会炸）；脚本内部以显式
 * argv 数组 spawn，无 shell 语义差异。
 *
 * 行为契约：
 *   - 枚举：pnpm ls -r --depth -1 --json，过滤 private
 *   - 守卫：GO/SKIP 行透传到 stderr；任一查询失败（guard exit 2）→ 不发起
 *     publish、本脚本 exit 2（原子性：绝不放行半个计划）
 *   - 全部已上架 → 打印 nothing to do、exit 0（不调用 pnpm publish）
 *   - 全新包名：GO 包逐个查包级文档，404 = 包名从未上架 → CI（OIDC 上下文）
 *     非 dry-run 整体拦停 exit 2 + 打印首发仪式指引；本地手动首发是仪式
 *     本身，仅警告不拦（ADR-0031 首发仪式触发绑定的闭环）
 *   - 有未上架 → pnpm [--filter <包名>]×N publish -r --no-git-checks [--dry-run]
 *     （--filter 置于全局位：publish 子命令位上多个 --filter 会被 pnpm 参数解析
 *     当成非法；pnpm 原生拓扑序——平台包 → core → 用户包——不受影响），
 *     透传 pnpm 退出码
 *
 * PUBLISH_GUARD_NPM_URL 同时导向守卫查询与 pnpm 的 --registry（测试 stub /
 * 自建 registry 通用）；未设置时走默认 registry（https://registry.npmjs.org）。
 * 认证：OIDC trusted publishing——CI 声明 id-token: write 后 pnpm 自动换
 * 短期 token（provenance 随 OIDC 自动附加），无需 ~/.npmrc 与长效 token。
 */
import { execFile, spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const guard = fileURLToPath(new URL('./publish-guard.ts', import.meta.url))
const dryRun = process.argv.includes('--dry-run')
const registryOverride = process.env.PUBLISH_GUARD_NPM_URL?.replace(/\/$/, '')
const GUARD_UA = 'vill-v-kit/bumpp publish-guard (https://github.com/vill-v-kit/bumpp)'

interface ExecResult {
  stdout: string
  stderr: string
}

interface ExecError extends Error {
  code?: number
  stdout?: string
  stderr?: string
}

function exec(cmd: string, args: string[]): Promise<ExecResult> {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { cwd: root }, (error, stdout, stderr) => {
      if (error) {
        const e = error as ExecError
        e.stdout = stdout
        e.stderr = stderr
        reject(e)
      } else {
        resolve({ stdout, stderr })
      }
    })
  })
}

// 1. 枚举可上架包：workspace 全部非 private 项目（name + version）
let listStdout: string
try {
  ;({ stdout: listStdout } = await exec('pnpm', ['ls', '-r', '--depth', '-1', '--json']))
} catch (err) {
  console.error(`ERROR pnpm ls failed: ${(err as Error).message}`)
  process.exit(2)
}
const publishable = (JSON.parse(listStdout) as { name: string; version: string; private?: boolean }[])
  .filter((p) => !p.private)

// 2. 逐包过守卫（并行；guard 输出透传到 stderr）
const results = await Promise.all(
  publishable.map(async (p) => {
    try {
      const { stdout } = await exec('node', [guard, 'npm', p.name, p.version])
      process.stderr.write(stdout) // GO 行
      return { name: p.name, go: true }
    } catch (err) {
      const e = err as ExecError
      if (e.code === 1) {
        process.stderr.write(e.stdout ?? '') // SKIP 行
        return { name: p.name, go: false }
      }
      process.stderr.write(e.stderr || `ERROR guard crashed for ${p.name}: ${e.message}\n`)
      return { name: p.name, go: null }
    }
  }),
)

// 3. 原子性：任一查询失败则不发起 publish
if (results.some((r) => r.go === null)) {
  process.exit(2)
}

// 4. 全部已上架 → 收工
const todo = results.filter((r) => r.go).map((r) => r.name)
if (todo.length === 0) {
  console.log(`all ${results.length} packages already published — nothing to do`)
  process.exit(0)
}

// 5. 全新包名前置检测（首发仪式的触发绑定）：对 GO 包查包级文档（/latest
// 端点，响应小），404 = 包名从未上架。npm OIDC trusted publishing 要求包
// 已存在且已配置 trusted publisher，全新包名在 CI 必于拓扑序中段 404、
// 造成部分上架（v6.1.0 musl 实例）——CI 上整体拦停 exit 2；本地手动首发
// 是仪式本身，只警告不拦。查询失败同守卫语义 exit 2（不在未知态放行）
const npmBase = registryOverride ?? 'https://registry.npmjs.org'
const brandNew: string[] = []
let probeFailed = false
await Promise.all(
  todo.map(async (name) => {
    try {
      const res = await fetch(`${npmBase}/${encodeURIComponent(name)}/latest`, {
        headers: { 'user-agent': GUARD_UA },
      })
      await res.arrayBuffer() // 消费响应体，归还连接
      if (res.status === 404) {
        brandNew.push(name)
      } else if (res.status !== 200) {
        console.error(`ERROR npm package query failed: HTTP ${res.status} for ${name}`)
        probeFailed = true
      }
    } catch (err) {
      const e = err as Error & { cause?: { code?: string } }
      console.error(`ERROR npm package query failed: ${e.cause?.code ?? e.message} for ${name}`)
      probeFailed = true
    }
  }),
)
if (probeFailed) {
  process.exit(2)
}
if (brandNew.length > 0) {
  // dry-run 是验证不是上架，与本地一样只警告
  const blocking = process.env.CI && !dryRun
  console.error(`${blocking ? 'ERROR' : 'WARN'} 发布计划含从未上架的全新包名：
${brandNew.map((n) => `  - ${n}`).join('\n')}

npm trusted publishing 要求包已存在且已配置 trusted publisher，新包名没有
配置页，CI/OIDC 无法首发。请执行首发仪式（CONTEXT.md「首发仪式」）：
  1. 本地 pnpm login（OTP）后运行 node scripts/npm-publish.ts——非 CI
     环境只警告不拦停，守卫会过滤已上架包、只发未上架的（含全新包名）
  2. npmjs.com 各新包设置页配置 trusted publisher（vill-v-kit/bumpp +
     workflow 文件名）
  3. CI「Re-run failed jobs」，由 OIDC 收后续版本`)
  if (blocking) {
    process.exit(2)
  }
}

// 6. 放行：--filter 全局位 + pnpm 原生拓扑序
const args = [
  ...todo.flatMap((name) => ['--filter', name]),
  'publish',
  '-r',
  '--no-git-checks',
  ...(dryRun ? ['--dry-run'] : []),
  ...(registryOverride ? ['--registry', registryOverride] : []),
]
console.log(`run: pnpm ${args.join(' ')}`)
const child = spawn('pnpm', args, { cwd: root, stdio: 'inherit' })
child.on('exit', (code, signal) => {
  if (signal) {
    console.error(`ERROR pnpm publish terminated by ${signal}`)
    process.exit(2)
  }
  process.exit(code ?? 2)
})
