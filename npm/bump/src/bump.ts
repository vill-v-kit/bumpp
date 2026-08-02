import { versionBump, versionBumpInfo } from '@vill-v/bumpp-core'
import { Spinner } from 'picospinner'
import { changelog, getTag } from './changelog'
import { resolveConfig } from './config'
import type { BumpVersion, Config } from './types'

/**
 * 更新版本
 * @param option
 */
export const bumpVersion = async (option: Config = {}): Promise<BumpVersion> => {
  // 获取配置文件
  const config = await resolveConfig(option)
  // 获取远程仓库最新tag
  const currentTag = await getTag()
  // prompt 选择的版本信息
  const { state } = await versionBumpInfo()
  const res = { config } as BumpVersion
  res.bumpp = state
  // 如果远程仓库存在tag，才生成 changelog
  if (currentTag) {
    const spinner = new Spinner('changelog')
    spinner.start()
    try {
      res.changelog = await changelog({
        ...config.changelog,
        to: state.newVersion,
        from: state.currentVersion,
      })
      spinner.succeed('Update ' + config.changelog.output + ' success')
    } catch (error) {
      spinner.fail('Update ' + config.changelog.output + ' fail')
      throw error
    }
  }

  // 更新包版本信息（进度由 Rust Core 内置打印，ADR-0002）
  await versionBump({ ...config.bumpp, release: state.newVersion })

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
