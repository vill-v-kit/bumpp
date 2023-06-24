import { Config, bumpVersion as _bumpVersion, getCurrentGitBranch } from '@vill-v/bumpp'
import { isPreRelease, loadGiteeConfig, resolveRepoConfig } from './repo'
import { addRelease } from './open-api'
export const bumpVersion = async (option: Config = {}) => {
  const config = loadGiteeConfig()
  const repo = await resolveRepoConfig()
  if (!repo) {
    throw new Error('无法获取远程仓库信息')
  }
  const branch = await getCurrentGitBranch()
  const { bumpp, changelog } = await _bumpVersion(option)

  const { oraPromise } = await import('ora')
  await oraPromise(
    addRelease({
      access_token: config.access_token,
      name: bumpp.newVersion,
      tag_name: 'v' + bumpp.newVersion,
      body: changelog.markdown,
      target_commitish: branch,
      prerelease: isPreRelease(bumpp.newVersion),
      ...repo,
    }),
    {
      text: 'Gitee Release',
      successText: ' [Gitee] add release v' + bumpp.newVersion + ' success',
      failText: '[Gitee] add release v' + bumpp.newVersion + ' success',
    }
  )
}
