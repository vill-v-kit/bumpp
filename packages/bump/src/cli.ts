import cac from 'cac'
import { bumpVersion } from './bump'

declare const __version__: string

export const createCli = () => {
  cac('vbumpp')
    .command('[...files]')
    .option('-o,--output [output]', 'CHANGELOG.md 生成位置', { default: 'CHANGELOG.md' })
    .action(async (files, options) => {
      await bumpVersion({
        bumpp: {
          commit: true,
          tag: true,
          push: true,
          confirm: false,
          ignoreScripts: false,
          noVerify: false,
          files,
        },
        changelog: {
          output: options.output,
        },
      })
    })
    .cli.version(__version__)
    .help()
    .parse()
}
