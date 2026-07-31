import { type BumpVersion, type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { type ICreateGithubLikeRelease, createGithubLikeRelease } from '@vill-v/bumpp-github'
import consola from 'consola'

const createGiteeRelease: ICreateGithubLikeRelease = async (options: BumpVersion) => {
  const access_token = options.config.accesstoken?.gitee
  if (!access_token) {
    throw new Error('未检测到 Gitee token，请执行 vbumpp token set gitee 录入')
  }
  const createRelease = await createGithubLikeRelease({
    baseURL: 'https://gitee.com/api/v5',
    onRequest(context) {
      ;(context.options.body as Record<string, any>)!.access_token = access_token
    },
    async onResponseError({ response }) {
      consola.error('gitee [open api] error :', response._data?.message || '')
    },
  })

  await createRelease(options)
}

export const bumpVersion = async (option: Config = {}): Promise<void> =>
  bumpVersionWithBaseRelease(option, createGiteeRelease, 'Gitee')
