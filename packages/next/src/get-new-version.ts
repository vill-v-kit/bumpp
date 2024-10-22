import select from '@inquirer/select'
import { colors } from 'consola/utils'
import { RELEASE_TYPES, type ReleaseType, SemVer, inc } from 'semver'

function _semverNewVersion(currentVersion: SemVer, release_type: ReleaseType) {
  const identifier = (currentVersion.prerelease[0] as string) || 'beta'
  const newVersion = new SemVer(inc(currentVersion, release_type, identifier)!)
  if (!release_type.startsWith('pre')) {
    return newVersion.version
  }
  if (
    newVersion.prerelease.length &&
    newVersion.prerelease.length === 2 &&
    newVersion.prerelease[1] === 0
  ) {
    // @ts-expect-error
    newVersion.prerelease[1] = 1
    newVersion.format()
  }
  return newVersion.version
}

export function semverNewVersion(currentVersion: string) {
  const semver = new SemVer(currentVersion)
  const result: Record<string, string> = {}
  RELEASE_TYPES.forEach((key) => {
    result[key] = _semverNewVersion(semver, key)
  })
  result.next = semver.prerelease.length ? result.prerelease : result.patch
  if (semver.prerelease.length) {
    semver.prerelease = []
    semver.format()
    result.patch = semver.version
  }
  return result
}

export async function promptNewVersion(currentVersion: string) {
  const newVersion = semverNewVersion(currentVersion)
  const result = await select({
    message: `Current version ${colors.green(currentVersion)}`,
    default: 'next',
    choices: [
      { name: `major ${newVersion.major}`, value: 'major' },
      { name: `minor ${newVersion.minor}`, value: 'minor' },
      { name: `patch ${newVersion.patch}`, value: 'patch' },
      { name: `next ${newVersion.next}`, value: 'next' },
      { name: `pre-patch ${newVersion.prepatch}`, value: 'prepatch' },
      { name: `pre-minor ${newVersion.preminor}`, value: 'preminor' },
      { name: `pre-major ${newVersion.premajor}`, value: 'premajor' },
    ],
  })

  return result
}
