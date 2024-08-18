import { type BumpVersion, type Config, bumpVersionWithBaseRelease } from '@vill-v/bumpp'
import { type ICreateGithubLikeRelease, createGithubLikeRelease } from '@vill-v/bumpp-github'
import consola from 'consola'

const createGiteeRelease: ICreateGithubLikeRelease = async (options: BumpVersion) => {
  const access_token = options.config.accesstoken?.gitee
  const createRelease = await createGithubLikeRelease({
    baseURL: 'https://gitee.com/api/v5',
    body: {
      access_token,
    },
    async onResponseError({ response }) {
      consola.error('gitee [open api] error :', response._data?.message || '')
    },
  })

  await createRelease(options)
}

export const bumpVersion = async (option: Config = {}) =>
  bumpVersionWithBaseRelease(option, createGiteeRelease, 'Gitee')
