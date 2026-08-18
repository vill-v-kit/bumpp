import { bumpVersion as coreBumpVersion } from '@vill-v/bumpp-core'
import type { BumpVersion, Config } from './types'

/**
 * 更新版本（：编排全在 Rust Core——统一配置解析 → 交互选版本 →
 * changelog → 文件/脚本/git；本函数仅类型化透传，类型不符在 napi 边界
 * 即运行期报错）
 * @param option 扁平配置覆盖（与 .vbumpprc.* 同形）
 */
export const bumpVersion = (option: Config = {}): Promise<BumpVersion> =>
  coreBumpVersion(option)
