import { cac } from 'cac'
import { bumpVersion } from './bump'
import { loadGiteeConfig, resolveRepoConfig } from './repo'

declare const __version__: string

export const createCli = () => {
  const $cac = cac('vbumpp')
  $cac
    .command('[...files]', 'release tool')
    .option('-o,--output [output]', 'CHANGELOG.md 生成位置', { default: 'CHANGELOG.md' })
    .option('-r,--recursive', 'recursively', { default: false })
    .action(async (files, options) => {
      const config = await loadGiteeConfig()
      const repo = await resolveRepoConfig()
      if (!repo) {
        throw new Error('无法获取远程仓库信息')
      }
      await bumpVersion(
        {
          bumpp: {
            commit: true,
            tag: true,
            push: true,
            confirm: false,
            ignoreScripts: false,
            noVerify: false,
            files,
            recursive: options.recursive,
          },
          changelog: {
            output: options.output,
          },
        },
        {
          ...config,
          ...repo,
        }
      )
    })

  $cac.version(__version__)
  $cac.help()
  $cac.parse()
}
