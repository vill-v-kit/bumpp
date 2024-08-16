import { createBaseCli } from '@vill-v/bumpp/cli'
import { bumpVersion } from './bump'
import { loadGiteeConfig, resolveRepoConfig } from './repo'

export const createCli = () => {
  createBaseCli(async (baseConfig) => {
    const repo = await resolveRepoConfig()
    if (!repo) {
      throw new Error('无法获取远程仓库信息')
    }
    const config = await loadGiteeConfig()
    await bumpVersion(baseConfig, {
      ...config,
      ...repo,
    })
  })
}
