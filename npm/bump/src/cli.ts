import { tokenList, tokenRemove, tokenSet } from '@vill-v/bumpp-core'
import { cac } from 'cac'
import { consola } from 'consola'
import { version as __version__ } from '../package.json'
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
        // 空 files 省略——避免 overrides 整体替换掉配置文件的 files
        //（ADR-0013 浅合并语义；旧 defu 为数组拼接）
        ...(files.length ? { files } : {}),
        recursive: options.recursive,
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
          try {
            // 密码交互在 Rust 侧（dialoguer，ADR-0014）
            const saved = await tokenSet(name)
            if (saved) {
              consola.success(`${name} token 已加密保存`)
            } else {
              consola.warn('已取消录入')
            }
          } catch (error) {
            consola.error((error as Error).message)
            process.exitCode = 1
          }
          break
        }
        case 'list': {
          const tokens = tokenList()
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
          if (tokenRemove(name)) {
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
