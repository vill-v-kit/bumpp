import { type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { getCurrentGitBranch } from '@vill-v/bumpp/changelogen'
import { type BaseOpenApiOption, addRelease } from './open-api'
import { isPreRelease } from './repo'

export const bumpVersion = async (option: Config = {}, gitee: BaseOpenApiOption) => {
  const branch = await getCurrentGitBranch()

  await bumpVersionWithBaseRelease(
    option,
    ({ bumpp, changelog }) =>
      addRelease({
        name: bumpp.newVersion,
        tag_name: 'v' + bumpp.newVersion,
        body: changelog.markdown,
        target_commitish: branch,
        prerelease: isPreRelease(bumpp.newVersion),
        ...gitee,
      }),
    'Gitee'
  )
}
