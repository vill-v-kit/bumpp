import { type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { createGitlabRelease } from './createRelease'

export const bumpVersion = async (option: Config = {}): Promise<void> =>
  bumpVersionWithBaseRelease(option, createGitlabRelease, 'Gitlab')
