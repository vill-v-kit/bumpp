import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp'
import { consola } from 'consola'
import { read, readUser } from 'rc9'
import chalk from 'chalk'

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

export interface GiteeConfig {
  access_token: string
}

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

export const isPreRelease = (v: string) => {
  return /(beta|alpha)/.test(v)
}
