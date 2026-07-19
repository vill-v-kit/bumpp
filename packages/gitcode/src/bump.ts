import { type BumpVersion, type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { type ICreateGithubLikeRelease, createGithubLikeRelease } from '@vill-v/bumpp-github'
import consola from 'consola'

const createGitcodeRelease: ICreateGithubLikeRelease = async (options: BumpVersion) => {
  const access_token = options.config.accesstoken?.gitcode
  if (!access_token) {
    throw new Error('未检测到 GitCode token，请执行 vbumpp token set gitcode 录入')
  }
  const createRelease = await createGithubLikeRelease({
    baseURL: 'https://api.gitcode.com/api/v5',
    onRequest(context) {
      context.options.query = { ...context.options.query, access_token }
    },
    async onResponseError({ response }) {
      consola.error('gitcode [open api] error :', response._data?.message || '')
    },
  })

  await createRelease(options)
}

export const bumpVersion = async (option: Config = {}): Promise<void> =>
  bumpVersionWithBaseRelease(option, createGitcodeRelease, 'GitCode')
