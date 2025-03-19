import { cac } from 'cac'
import { bumpVersion } from './bump'
import type { Config } from './types'

declare const __version__: string
export const createBaseCli = (
  bumpVersion: (config?: Config) => Promise<any>,
  version?: string
): void => {
  const $cac = cac('vbumpp')
  $cac
    .command('[...files]')
    .option('-o,--output [output]', 'CHANGELOG.md 生成位置', { default: 'CHANGELOG.md' })
    .option('-r,--recursive', 'recursively', { default: false })
    .action(async (files, options) => {
      await bumpVersion({
        bumpp: {
          files,
          recursive: options.recursive,
        },
        changelog: {
          output: options.output,
        },
      })
    })

  $cac.version(version || __version__)
  $cac.help()
  $cac.parse()
}

export const createCli = (): void => createBaseCli(bumpVersion)
