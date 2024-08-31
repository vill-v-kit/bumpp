import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp/changelogen'
import consola from 'consola'
import { colors } from 'consola/utils'

/**
 * 加载gitee远程仓库信息
 */
export const resolveRepoConfig = async () => {
  const { repo: _repo, domain } = await _resolveRepoConfig(process.cwd())
  if (!_repo) {
    throw new Error('无法获取远程仓库信息')
  }
  const [owner, repo] = _repo.split('/')
  consola.info(
    'repo:',
    colors.bold('domain'),
    colors.bold(colors.green(domain || 'unknown')),
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
