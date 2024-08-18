import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp/changelogen'
import consola from 'consola'
import { colors } from 'consola/utils'

/**
 * 加载gitee远程仓库信息
 */
export const resolveRepoConfig = async () => {
  const { repo: _repo } = await _resolveRepoConfig(process.cwd())
  if (!_repo) {
    throw new Error('无法获取远程仓库信息')
  }
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

/**
 * 判断当前版本号是够使 预发布版本
 * @param v
 */
export const isPreRelease = (v: string) => {
  return /(beta|alpha)/.test(v)
}
