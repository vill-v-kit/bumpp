import { ProgressEvent, VersionBumpProgress, versionBump } from 'bumpp'
import { changelog, getTag } from './changelog'
import { resolveConfig } from './config'
import { Config } from './types'
import consola from 'consola'

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

export const bumpVersion = async (option: Config = {}) => {
  const config = await resolveConfig(option)
  const currentTag = await getTag()
  const options = await versionBump({ ...config.bumpp, progress })

  if (!currentTag) {
    return {
      changelogMD: '',
      markdown: '',
    }
  }

  const { changelogMD, markdown } = await changelog({
    ...config.changelog,
    to: options.newVersion,
    from: options.oldVersion,
  })

  return {
    changelogMD,
    markdown,
  }
}
