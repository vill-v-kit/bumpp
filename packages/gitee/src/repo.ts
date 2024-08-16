import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp/changelogen'
import { consola } from 'consola'
import { colors } from 'consola/utils'
import { read, readUser } from 'rc9'

/**
 * 加载gitee远程仓库信息
 */
export const resolveRepoConfig = async () => {
  const { repo: _repo } = await _resolveRepoConfig(process.cwd())
  if (_repo) {
    const [owner, repo] = _repo.split('/')
    consola.info(
      'gitee repo:',
      colors.bold('owner'),
      colors.bold(colors.green(owner)),
      colors.bold('repo'),
      colors.bold(colors.green(repo))
    )
    return {
      owner,
      repo,
    }
  }
}

/**
 * .giteerc 配置信息
 */
export interface GiteeConfig {
  /**
   * open-api 调用凭证
   */
  access_token: string
}

/**
 * 加载 gitee 配置
 */
export const loadGiteeConfig = async () => {
  const config = read<GiteeConfig>('.giteerc')
  const userConfig = readUser<GiteeConfig>('.giteerc')
  if (Object.keys(config).length) {
    consola.info(
      'gitee config:',
      colors.bold('access_token'),
      colors.bold(colors.green(config.access_token))
    )
    return config
  }
  consola.info(
    'gitee config [global]:',
    colors.bold('access_token'),
    colors.bold(colors.green(userConfig.access_token))
  )
  return userConfig
}

/**
 * 判断当前版本号是够使 预发布版本
 * @param v
 */
export const isPreRelease = (v: string) => {
  return /(beta|alpha)/.test(v)
}
