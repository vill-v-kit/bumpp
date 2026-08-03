import {
  generateChangelog,
  getLastGitTag,
  loadBumpConfig,
  versionBump,
  versionBumpInfo,
} from '@vill-v/bumpp-core'
import { Spinner } from 'picospinner'
import { consola } from 'consola'
import { readTokenStore } from './accesstoken'
import type { Accesstokens, BumpVersion, Config } from './types'

/**
 * 更新版本
 * @param option 扁平配置覆盖（与 .vbumpprc.json 同形，ADR-0013）
 */
export const bumpVersion = async (option: Config = {}): Promise<BumpVersion> => {
  // 配置统一由 Rust 单一解析路径解析（overrides > .vbumpprc.json > 内建默认）；
  // confirm 缺省关闭——版本经 versionBumpInfo 交互选择，不再二次确认
  const bumpp = loadBumpConfig({ confirm: false, ...option })
  // accesstoken 留 JS（ADR-0013：加密凭证存储，非配置文件加载）
  let accesstoken: Accesstokens = {}
  try {
    accesstoken = await readTokenStore()
  } catch {
    consola.warn('token 存储文件读取失败（设备或用户已变更），请重新执行 vbumpp token set <name>')
  }
  // 获取远程仓库最新 tag（真实 tag 名，ADR-0012 C1）
  const currentTag = getLastGitTag()
  // prompt 选择的版本信息
  const { state } = await versionBumpInfo()
  const res = {
    bumpp: state,
    config: { bumpp, changelog: option.changelog ?? {}, accesstoken },
  } as BumpVersion
  // 如果远程仓库存在tag，才生成 changelog
  if (currentTag) {
    // spinner 文案路径自 overrides 取（文件级 output 的解析在 Rust 内部，此处尽力而为）
    const output = option.changelog?.output ?? 'CHANGELOG.md'
    const spinner = new Spinner('changelog')
    spinner.start()
    try {
      res.changelog = generateChangelog({
        overrides: option,
        from: currentTag,
        to: state.newVersion,
      })
      spinner.succeed('Update ' + output + ' success')
    } catch (error) {
      spinner.fail('Update ' + output + ' fail')
      throw error
    }
  }

  // 更新包版本信息（进度由 Rust Core 内置打印，ADR-0002）
  await versionBump({ ...bumpp, release: state.newVersion })

  return res
}

/**
 * 更新版本伴随基础 release 基础等待动画
 * @param option 更新版本配置
 * @param addRelease release脚本
 * @param provider git 远程储存提供商
 */
export const bumpVersionWithBaseRelease = async (
  option: Config | undefined = {},
  addRelease: (res: BumpVersion) => Promise<any>,
  provider: string
): Promise<void> => {
  const { bumpp, changelog, config } = await bumpVersion(option)

  const spinner = new Spinner(`${provider} Release`)
  spinner.start()
  try {
    await addRelease({ bumpp, changelog, config })
    spinner.succeed(`[${provider}] add release v` + bumpp.newVersion + ' success')
  } catch (error) {
    spinner.fail(`[${provider}] add release v` + bumpp.newVersion + ' fail')
    throw error
  }
}
