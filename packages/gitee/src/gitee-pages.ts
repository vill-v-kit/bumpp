import { BaseOpenApiOption, updateDocPages } from './open-api'

/**
 * 使 gitee-pages 重新构建
 */
export const giteePagesBuild = async (gitee: BaseOpenApiOption) => {
  const { oraPromise } = await import('ora')

  await oraPromise(updateDocPages(gitee), {
    text: 'Gitee Pages Build',
    successText: ' [Gitee] pages build request success',
    failText: '[Gitee] pages build request success',
  })
}
