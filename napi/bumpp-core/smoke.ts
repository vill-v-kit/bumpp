// napi 链路冒烟（三收缩后的导出面）：编排与 CLI 单入口可调用，
// 平台 Release 四导出、旧 parity 面与 token 三件套不再导出
import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { bumpVersion, cliRun } from './index.js'

const dir = mkdtempSync(join(tmpdir(), 'bumpp-smoke-'))
// token 存储与全局配置目录指向临时路径，不碰真实 ~/.vbumpp
process.env.VBUMPP_TOKEN_STORE = join(dir, 'tokens.bin')
process.env.VBUMPP_HOME = join(dir, 'home')

const checks: [string, boolean][] = [
  ['bumpVersion export exists', typeof bumpVersion === 'function'],
  ['cliRun export exists', typeof cliRun === 'function'],
]

// provider 校验在编排入口同步失败（不经网络/交互）
try {
  await bumpVersion({}, 'bogus', dir)
  checks.push(['bumpVersion rejects unknown provider', false])
} catch (error) {
  checks.push([
    'bumpVersion rejects unknown provider',
    String(error).includes('unknown provider: bogus'),
  ])
}

// 类型化边界：类型不符在 napi 边界即运行期错误（静默回落通路消除）。
// 类型层经 @ts-expect-error 之外的显式 cast 绕过，模拟运行期传入错值
const badOverrides = { commit: 123 } as unknown as Parameters<typeof bumpVersion>[0]
try {
  await bumpVersion(badOverrides, undefined, dir)
  checks.push(['bumpVersion rejects type mismatch (commit: 123)', false])
} catch (error) {
  const message = String(error)
  checks.push([
    'bumpVersion rejects type mismatch (commit: 123)',
    message.includes('commit') && !message.includes('unknown provider'),
  ])
}

// CLI 通路（argv 全权归 Rust，返回退出码）
checks.push(['cliRun --version exits 0', (await cliRun(['--version'])) === 0])
checks.push(['cliRun --help exits 0', (await cliRun(['--help'])) === 0])
checks.push(['cliRun token list on empty store exits 0', (await cliRun(['token', 'list'])) === 0])
checks.push(['cliRun token remove (absent) exits 0', (await cliRun(['token', 'remove', 'github'])) === 0])
checks.push(['cliRun token unknown action exits 1', (await cliRun(['token', 'peek'])) === 1])
checks.push(['cliRun unknown option exits 1', (await cliRun(['--wat'])) === 1])
checks.push(['cliRun schema exits 0', (await cliRun(['schema'])) === 0])

// bump 通路：空目录直达编排层首错（临时 cwd，不经交互）
const cwd = process.cwd()
process.chdir(dir)
checks.push(['cliRun bump in empty dir exits 1', (await cliRun([])) === 1])
checks.push(['cliRun bump with unknown provider exits 1', (await cliRun([], 'bogus')) === 1])
// schema --write 落点默认项目级（临时 cwd，不碰真实工作区）
checks.push(['cliRun schema --write exits 0', (await cliRun(['schema', '--write'])) === 0])
checks.push(['cliRun schema --write lands vbumpprc.schema.json', existsSync(join(dir, 'vbumpprc.schema.json'))])
process.chdir(cwd)

// 四个平台变体 bin 的 provider 注入锚定：空目录跑 bin，退出码 1 且 stderr
// 不得出现「未知 provider」（字面量写错时解析层就会报，语义化钉死）
const variantBins = ['github', 'gitlab', 'gitee', 'gitcode']
for (const provider of variantBins) {
  const bin = new URL(`../../npm/${provider}/bin/index.js`, import.meta.url).pathname
  const result = spawnSync(process.execPath, [bin], {
    cwd: dir,
    env: {
      PATH: process.env.PATH,
      VBUMPP_TOKEN_STORE: process.env.VBUMPP_TOKEN_STORE,
      VBUMPP_HOME: process.env.VBUMPP_HOME,
    },
    encoding: 'utf8',
  })
  checks.push([
    `variant bin (${provider}) injects its provider`,
    result.status === 1 && !result.stderr.includes('unknown provider'),
  ])
}

let failed = 0
for (const [name, ok] of checks) {
  console.log(ok ? '✓' : '✗', name)
  if (!ok) failed++
}
rmSync(dir, { recursive: true, force: true })
process.exit(failed)
