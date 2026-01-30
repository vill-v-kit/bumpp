import { type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { createGitlabRelease } from './createRelease'

/// 2f
export const bumpVersion = async (option: Config = {}): Promise<void> =>
  bumpVersionWithBaseRelease(option, createGitlabRelease, 'Gitlab')
