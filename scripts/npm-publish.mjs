#!/usr/bin/env node
/**
 * npm 上架（COL-53，ADR-0021 决策⑤的消费侧）：枚举 workspace 全部非 private 包
 * （当前恰好 11 个——website 与根 monorepo 为 private 自动排除），逐一过
 * publish-guard（COL-51）查询 registry，只对未上架的包执行 `pnpm publish`。
 * 已上架包跳过、查询失败整体不放行——「Re-run failed jobs」重跑即收敛。
 *
 * 用法（ci.yml publish-npm job）：
 *   NODE_AUTH_TOKEN=… node scripts/npm-publish.mjs            # 实际上架
 *   node scripts/npm-publish.mjs --dry-run                    # 干跑（本地/CI 验证，不上传）
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
 *   - 有未上架 → pnpm [--filter <包名>]×N publish -r --no-git-checks [--dry-run]
 *     （--filter 置于全局位：publish 子命令位上多个 --filter 会被 pnpm 参数解析
 *     当成非法；pnpm 原生拓扑序——平台包 → core → 用户包——不受影响），
 *     透传 pnpm 退出码
 *
 * PUBLISH_GUARD_NPM_URL 同时导向守卫查询与 pnpm 的 --registry（测试 stub /
 * 自建 registry 通用）；未设置时走默认 registry（https://registry.npmjs.org）。
 * 认证由调用方准备（CI 在 ~/.npmrc 写 ${NODE_AUTH_TOKEN} 展开式）。
 */
import { execFile, spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const guard = fileURLToPath(new URL('./publish-guard.mjs', import.meta.url))
const dryRun = process.argv.includes('--dry-run')
const registryOverride = process.env.PUBLISH_GUARD_NPM_URL?.replace(/\/$/, '')

function exec(cmd, args) {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { cwd: root }, (error, stdout, stderr) => {
      if (error) {
        error.stdout = stdout
        error.stderr = stderr
        reject(error)
      } else {
        resolve({ stdout, stderr })
      }
    })
  })
}

// 1. 枚举可上架包：workspace 全部非 private 项目（name + version）
let listStdout
try {
  ;({ stdout: listStdout } = await exec('pnpm', ['ls', '-r', '--depth', '-1', '--json']))
} catch (err) {
  console.error(`ERROR pnpm ls failed: ${err.message}`)
  process.exit(2)
}
const publishable = JSON.parse(listStdout).filter((p) => !p.private)

// 2. 逐包过守卫（并行；guard 输出透传到 stderr）
const results = await Promise.all(
  publishable.map(async (p) => {
    try {
      const { stdout } = await exec('node', [guard, 'npm', p.name, p.version])
      process.stderr.write(stdout) // GO 行
      return { name: p.name, go: true }
    } catch (err) {
      if (err.code === 1) {
        process.stderr.write(err.stdout) // SKIP 行
        return { name: p.name, go: false }
      }
      process.stderr.write(err.stderr || `ERROR guard crashed for ${p.name}: ${err.message}\n`)
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

// 5. 放行：--filter 全局位 + pnpm 原生拓扑序
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
