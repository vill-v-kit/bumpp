import { ofetch } from 'ofetch'
import { consola } from 'consola'
const $fetch = ofetch.create({
  baseURL: 'https://gitee.com/api/v5',
  async onResponseError({ response }) {
    consola.error('gitee [open api] error :', response._data?.message || '')
  },
})

/**
 * gitee openapi 基础 传参
 */
export interface BaseOpenApiOption {
  /**
   * 仓库所属空间地址(企业、组织或个人的地址path)
   */
  owner: string
  /**
   * 仓库路径(path)
   */
  repo: string
  /**
   * 用户授权码
   */
  access_token: string
}

/**
 * 新增 Release 传参
 */
export interface AddReleaseOption extends BaseOpenApiOption {
  /**
   * Tag 名称, 提倡以v字母为前缀做为Release名称，例如v1.0或者v2.3.4
   */
  tag_name: string
  /**
   * Release 名称
   */
  name: string
  /**
   * Release 描述
   */
  body: string
  /**
   * 是否为预览版本。默认: false（非预览版本）
   */
  prerelease?: boolean
  /**
   * 分支名称或者commit SHA, 默认是当前默认分支
   */
  target_commitish?: string
}

/**
 * 创建仓库Release
 * @link https://gitee.com/api/v5/swagger#/postV5ReposOwnerRepoReleases
 * @param option
 */
export const addRelease = (option: AddReleaseOption) => {
  const { owner, repo, ...body } = option
  return $fetch(`/repos/${owner}/${repo}/releases`, { method: 'POST', body })
}

/**
 * 请求建立Pages 传参
 */
export interface UpdateDocPagesOption extends BaseOpenApiOption {}
