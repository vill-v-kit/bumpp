import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp'
import { consola } from 'consola'
import { read, readUser } from 'rc9'
import chalk from 'chalk'

/**
 * 加载gitee远程仓库信息
 */
export const resolveRepoConfig = async () => {
  const { repo: _repo } = await _resolveRepoConfig(process.cwd())
  if (_repo) {
    const [owner, repo] = _repo.split('/')
    consola.info(
      'gitee repo:',
      chalk.bold('owner'),
      chalk.green.bold(owner),
      chalk.bold('repo'),
      chalk.green.bold(repo)
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
    consola.info('gitee config:', chalk.bold('access_token'), chalk.green.bold(config.access_token))
    return config
  }
  consola.info(
    'gitee config [global]:',
    chalk.bold('access_token'),
    chalk.green.bold(userConfig.access_token)
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
