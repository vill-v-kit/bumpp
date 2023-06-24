import { type Config, bumpVersion as _bumpVersion, getCurrentGitBranch } from '@vill-v/bumpp'
import { type BaseOpenApiOption, addRelease } from './open-api'
import { isPreRelease } from './repo'
export const bumpVersion = async (option: Config = {}, gitee: BaseOpenApiOption) => {
  const branch = await getCurrentGitBranch()
  const { bumpp, changelog } = await _bumpVersion(option)

  const { oraPromise } = await import('ora')

  await oraPromise(
    addRelease({
      name: bumpp.newVersion,
      tag_name: 'v' + bumpp.newVersion,
      body: changelog.markdown,
      target_commitish: branch,
      prerelease: isPreRelease(bumpp.newVersion),
      ...gitee,
    }),
    {
      text: 'Gitee Release',
      successText: ' [Gitee] add release v' + bumpp.newVersion + ' success',
      failText: '[Gitee] add release v' + bumpp.newVersion + ' success',
    }
  )
}
