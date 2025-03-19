import type { BumpVersion } from '@vill-v/bumpp'
import { getCurrentGitBranch } from '@vill-v/bumpp/changelogen'
import { type FetchOptions, ofetch } from 'ofetch'
import consola from 'consola'
import { x } from 'tinyexec'
import { isPreRelease, resolveRepoConfig } from './utils'
interface GitHubLikeOpenApiCreateRelease {
  /**
   * Tag 名称, 提倡以v字母为前缀做为Release名称，例如v1.0或者v2.3.4
   */
  tag_name: string
  /**
   * Release 名称
   */
  name?: string
  /**
   * Release 描述
   */
  body?: string
  /**
   * 是否为预览版本。默认: false（非预览版本）
   */
  prerelease?: boolean
  /**
   * 分支名称或者commit SHA, 默认是当前默认分支
   */
  target_commitish?: string
}

export const createGithubLikeRelease = async (fetchOptions: FetchOptions): Promise<any> => {
  const repo = await resolveRepoConfig()

  const $fetch = ofetch.create(fetchOptions)

  const branch = await getCurrentGitBranch()

  return ({ bumpp, changelog }: BumpVersion) => {
    return $fetch(`/repos/${repo.owner}/${repo.repo}/releases`, {
      method: 'POST',
      body: <GitHubLikeOpenApiCreateRelease>{
        name: bumpp.newVersion,
        tag_name: 'v' + bumpp.newVersion,
        body: changelog?.markdown || '',
        target_commitish: branch,
        prerelease: isPreRelease(bumpp.newVersion),
      },
    })
  }
}

export interface ICreateGithubLikeRelease {
  (options: BumpVersion): Promise<void>
}

const resolveGithubCliToken = async (): Promise<string | undefined> => {
  try {
    const res = await x('gh', ['auth', 'token'])
    return res.stdout
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
  } catch (e) {
    return
  }
}

export const createGithubRelease: ICreateGithubLikeRelease = async (options: BumpVersion) => {
  let accesstoken =
    options.config.accesstoken?.github || process.env.GH_TOKEN || process.env.GITHOB_TOKEN
  if (!accesstoken) {
    accesstoken = await resolveGithubCliToken()
  }
  const createRelease = await createGithubLikeRelease({
    baseURL: 'https://api.github.com',
    headers: {
      'x-github-api-version': '2022-11-28',
      authorization: `Bearer ${accesstoken}`,
    },
    onRequestError(context) {
      const status = context.response?.status

      switch (status) {
        case 404:
          consola.error(
            'github [open api] error :',
            'Not Found if the discussion category name is invalid'
          )
          break
        case 422:
          consola.error(
            'github [open api] error :',
            'Validation failed, or the endpoint has been spammed.'
          )
          break
        default:
          consola.error('github [open api] error :', 'Unknown error')
          break
      }
    },
  })

  await createRelease(options)
}
