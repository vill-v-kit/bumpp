import { ProgressEvent, type VersionBumpProgress, versionBump, versionBumpInfo } from 'bumpp'
import { consola } from 'consola'
import { oraPromise } from 'ora'
import { changelog, getTag } from './changelog'
import { resolveConfig } from './config'
import { BumpVersion, Config } from './types'

/**
 * antfu/bumpp progress
 */
function progress({
  event,
  script,
  updatedFiles,
  skippedFiles,
  newVersion,
}: VersionBumpProgress): void {
  switch (event) {
    case ProgressEvent.FileUpdated:
      consola.success(`Updated ${updatedFiles.pop()} to ${newVersion}`)
      break

    case ProgressEvent.FileSkipped:
      consola.info(`${skippedFiles.pop()} did not need to be updated`)
      break

    case ProgressEvent.GitCommit:
      consola.info('Git commit')
      break

    case ProgressEvent.GitTag:
      consola.info('Git tag')
      break

    case ProgressEvent.GitPush:
      consola.success('Git push')
      break

    case ProgressEvent.NpmScript:
      consola.success(`Npm run ${script}`)
      break
  }
}

/**
 * 更新版本
 * @param option
 */
export const bumpVersion = async (option: Config = {}) => {
  // 获取配置文件
  const config = await resolveConfig(option)
  // 获取远程仓库最新tag
  const currentTag = await getTag()
  // prompt 选择的版本信息
  const { state } = await versionBumpInfo()
  const res = {} as BumpVersion
  res.bumpp = state
  // 如果远程仓库存在tag，才生成 changelog
  if (currentTag) {
    res.changelog = await oraPromise(
      changelog({
        ...config.changelog,
        to: state.newVersion,
        from: state.currentVersion,
      }),
      {
        text: 'changelog',
        successText: 'Update ' + config.changelog.output + ' success',
        failText: 'Update ' + config.changelog.output + ' success',
      }
    )
  }

  // 更新包版本信息
  await versionBump({ ...config.bumpp, progress, release: state.newVersion })

  return res
}

/**
 * 更新版本伴随基础 release 基础等待动画
 * @param option 更新版本配置
 * @param addRelease release脚本
 * @param provider git 远程储存提供商
 */
export const bumpVersionWithBaseRelease = async (
  option: Config = {},
  addRelease: (res: BumpVersion) => Promise<any>,
  provider: string
) => {
  const { bumpp, changelog } = await bumpVersion(option)

  await oraPromise(addRelease({ bumpp, changelog }), {
    text: `${provider} Release`,
    successText: ` [${provider}] add release v` + bumpp.newVersion + ' success',
    failText: `[${provider}] add release v` + bumpp.newVersion + ' success',
  })
}
