// napi 链路冒烟（ADR-0014 收缩后的导出面）：新编排/Release/token 导出可调用，
// 旧 parity 面不再导出
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  bumpVersion,
  createGitcodeRelease,
  createGiteeRelease,
  createGithubRelease,
  createGitlabRelease,
  tokenList,
  tokenRemove,
  tokenSet,
} from './index.js'

const dir = mkdtempSync(join(tmpdir(), 'bumpp-smoke-'))
// token 存储指向临时路径，不碰真实 ~/.vbumpp
process.env.VBUMPP_TOKEN_STORE = join(dir, 'tokens.bin')

const checks = [
  ['bumpVersion 导出存在', typeof bumpVersion === 'function'],
  ['createGithubRelease 导出存在', typeof createGithubRelease === 'function'],
  ['createGitlabRelease 导出存在', typeof createGitlabRelease === 'function'],
  ['createGiteeRelease 导出存在', typeof createGiteeRelease === 'function'],
  ['createGitcodeRelease 导出存在', typeof createGitcodeRelease === 'function'],
  ['tokenSet 导出存在', typeof tokenSet === 'function'],
  ['tokenList 空存储返回空表', Array.isArray(tokenList()) && tokenList().length === 0],
  ['tokenRemove 空存储返回 false', tokenRemove('github') === false],
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

let failed = 0
for (const [name, ok] of checks) {
  console.log(ok ? '✓' : '✗', name)
  if (!ok) failed++
}
rmSync(dir, { recursive: true, force: true })
process.exit(failed)
