import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp/changelogen'
import consola from 'consola'
import { colors } from 'consola/utils'
import type { $Fetch } from 'ofetch'

/**
 * 加载gitee远程仓库信息
 */
export const resolveRepoConfig = async ($fetch: $Fetch) => {
  const { repo: _repo, domain } = await _resolveRepoConfig(process.cwd())
  if (!_repo) {
    throw new Error('无法获取远程仓库信息')
  }

  const data = await $fetch<any[]>('/api/v4/projects', {
    method: 'GET',
    query: { search: _repo, search_namespaces: true, simple: true },
  })

  const id = data.find((item) => new URL(item.web_url).pathname.endsWith(_repo))?.id
  if (!id) {
    throw new Error('无法获取项目对应 gitlab 项目 id')
  }
  const [owner, repo] = _repo.split('/')

  consola.info(
    'repo:',
    colors.bold('domain'),
    colors.bold(colors.green(domain || 'unknown')),
    colors.bold('owner'),
    colors.bold(colors.green(owner)),
    colors.bold('repo'),
    colors.bold(colors.green(repo)),
    colors.bold('id'),
    colors.bold(colors.green(id))
  )

  return {
    owner,
    repo,
    id,
  }
}
