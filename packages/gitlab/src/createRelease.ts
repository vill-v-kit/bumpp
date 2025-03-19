import type { BumpVersion } from '@vill-v/bumpp'
import consola from 'consola'
import { ofetch } from 'ofetch'
import { resolveRepoConfig } from './utils'
interface GitlabOpenApiCreateRelease {
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
  description?: string
}

export const createGitlabRelease = async (options: BumpVersion): Promise<any> => {
  const accesstoken = options.config.accesstoken?.gitlab || ''
  const host = options.config.gitlab?.host || 'https://gitlab.com'
  const $fetch = ofetch.create({
    baseURL: host,
    headers: {
      'PRIVATE-TOKEN': accesstoken,
    },
    onRequestError(context) {
      const status = context.response?.status
      const data = context.response?._data
      consola.error('gitlab [open api] error :', `[${status}]`, data?.message || 'Unknown error')
    },
  })
  const repo = await resolveRepoConfig($fetch)

  const { bumpp, changelog } = options

  return $fetch(`/api/v4/projects/${repo.id}/releases`, {
    method: 'POST',
    body: <GitlabOpenApiCreateRelease>{
      name: bumpp.newVersion,
      tag_name: 'v' + bumpp.newVersion,
      description: changelog?.markdown || '',
    },
  })
}
