import { type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { createGithubRelease } from './createRelease'

export const bumpVersion = async (option: Config = {}) =>
  bumpVersionWithBaseRelease(option, (info) => createGithubRelease(info), 'Github')
