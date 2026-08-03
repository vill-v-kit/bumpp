// napi 链路冒烟（ADR-0016 收缩后的导出面）：编排/Release/CLI 单入口可调用，
// 旧 parity 面与 token 三件套不再导出
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import {
  bumpVersion,
  cliRun,
  createGitcodeRelease,
  createGiteeRelease,
  createGithubRelease,
  createGitlabRelease,
} from './index.js'

const dir = mkdtempSync(join(tmpdir(), 'bumpp-smoke-'))
// token 存储与全局配置目录指向临时路径，不碰真实 ~/.vbumpp
process.env.VBUMPP_TOKEN_STORE = join(dir, 'tokens.bin')
process.env.VBUMPP_HOME = join(dir, 'home')

const checks = [
  ['bumpVersion 导出存在', typeof bumpVersion === 'function'],
  ['cliRun 导出存在', typeof cliRun === 'function'],
  ['createGithubRelease 导出存在', typeof createGithubRelease === 'function'],
  ['createGitlabRelease 导出存在', typeof createGitlabRelease === 'function'],
  ['createGiteeRelease 导出存在', typeof createGiteeRelease === 'function'],
  ['createGitcodeRelease 导出存在', typeof createGitcodeRelease === 'function'],
]

// provider 校验在编排入口同步失败（不经网络/交互）
try {
  await bumpVersion({}, 'bogus', dir)
  checks.push(['bumpVersion 未知 provider 报错', false])
} catch (error) {
  checks.push([
    'bumpVersion 未知 provider 报错',
    String(error).includes('未知 provider: bogus'),
  ])
}

// CLI 通路（ADR-0016：argv 全权归 Rust，返回退出码）
checks.push(['cliRun --version 退出码 0', (await cliRun(['--version'])) === 0])
checks.push(['cliRun --help 退出码 0', (await cliRun(['--help'])) === 0])
checks.push(['cliRun token list 空存储退出码 0', (await cliRun(['token', 'list'])) === 0])
checks.push(['cliRun token remove 未找到退出码 0', (await cliRun(['token', 'remove', 'github'])) === 0])
checks.push(['cliRun token 未知 action 退出码 1', (await cliRun(['token', 'peek'])) === 1])
checks.push(['cliRun 未知选项退出码 1', (await cliRun(['--wat'])) === 1])

// bump 通路：空目录直达编排层首错（临时 cwd，不经交互）
const cwd = process.cwd()
process.chdir(dir)
checks.push(['cliRun bump 空目录退出码 1', (await cliRun([])) === 1])
checks.push(['cliRun bump 未知 provider 退出码 1', (await cliRun([], 'bogus')) === 1])
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
    `变体 bin（${provider}）provider 注入生效`,
    result.status === 1 && !result.stderr.includes('未知 provider'),
  ])
}

let failed = 0
for (const [name, ok] of checks) {
  console.log(ok ? '✓' : '✗', name)
  if (!ok) failed++
}
rmSync(dir, { recursive: true, force: true })
process.exit(failed)
