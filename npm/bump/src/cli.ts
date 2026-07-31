import password from '@inquirer/password'
import { cac } from 'cac'
import { consola } from 'consola'
import { version as __version__ } from '../package.json'
import { readTokenStore, removeToken, saveToken } from './accesstoken'
import { bumpVersion } from './bump'
import type { Config } from './types'

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

  $cac
    .command(
      'token <action> [name]',
      '管理 token（action: set / list / remove），加密安全存储'
    )
    .action(async (action: string, name?: string) => {
      switch (action) {
        case 'set': {
          if (!name) {
            consola.error('用法: vbumpp token set <name>')
            process.exitCode = 1
            return
          }
          let input: string
          try {
            input = (await password({ message: `请输入 ${name} 的 access_token: ` })).trim()
          } catch {
            // 用户取消（Ctrl+C）或输入流被关闭
            consola.warn('已取消录入')
            return
          }
          if (!input) {
            consola.error('token 不能为空')
            process.exitCode = 1
            return
          }
          await saveToken(name, input)
          consola.success(`${name} token 已加密保存`)
          break
        }
        case 'list': {
          const tokens = Object.keys(await readTokenStore())
          if (!tokens.length) {
            consola.info('尚未配置任何 token')
            return
          }
          for (const name of tokens) {
            consola.info(name)
          }
          break
        }
        case 'remove': {
          if (!name) {
            consola.error('用法: vbumpp token remove <name>')
            process.exitCode = 1
            return
          }
          if (await removeToken(name)) {
            consola.success(`${name} token 已删除`)
          } else {
            consola.warn(`未找到 ${name} 的 token`)
          }
          break
        }
        default:
          consola.error(`未知 action: ${action}，可用: set / list / remove`)
          process.exitCode = 1
      }
    })

  $cac.version(version || __version__)
  $cac.help()
  $cac.parse()
}

export const createCli = (): void => createBaseCli(bumpVersion)
