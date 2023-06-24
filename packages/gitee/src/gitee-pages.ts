import { BaseOpenApiOption, updateDocPages } from './open-api'
import { oraPromise } from 'ora'
/**
 * 使 gitee-pages 重新构建
 */
export const giteePagesBuild = async (gitee: BaseOpenApiOption) => {
  await oraPromise(updateDocPages(gitee), {
    text: 'Gitee Pages Build',
    successText: ' [Gitee] pages build request success',
    failText: '[Gitee] pages build request success',
  })
}
