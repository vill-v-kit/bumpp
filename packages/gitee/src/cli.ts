import cac from 'cac'
import { bumpVersion } from './bump'
import { giteePagesBuild } from './gitee-pages'
import { loadGiteeConfig, resolveRepoConfig } from './repo'
declare const __version__: string

export const createCli = () => {
  const $cac = cac('vbumpp')
  $cac
    .command('[...files]', 'release tool')
    .option('-o,--output [output]', 'CHANGELOG.md 生成位置', { default: 'CHANGELOG.md' })
    .option('-r,--recursive', 'recursively', { default: false })
    .option('-p,--pages', '是否在 release 后请求更新本仓库的 gitee pages', { default: false })
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
      if (options.pages) {
        await giteePagesBuild({
          ...config,
          ...repo,
        })
      }
    })

  $cac.command('pages build', '请求构建 gitee pages').action(async () => {
    const config = await loadGiteeConfig()
    const repo = await resolveRepoConfig()
    if (!repo) {
      throw new Error('无法获取远程仓库信息')
    }
    await giteePagesBuild({
      ...config,
      ...repo,
    })
  })

  $cac.version(__version__).help().parse()
}
