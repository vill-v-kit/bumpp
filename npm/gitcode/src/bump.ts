import { bumpVersion as coreBumpVersion } from '@vill-v/bumpp-core'
import type { BumpVersion, Config } from '@vill-v/bumpp'

/**
 * 更新版本并创建 GitCode release（编排与 release 全在 Rust Core）
 */
export const bumpVersion = (option: Config = {}): Promise<BumpVersion> =>
  coreBumpVersion(option, 'gitcode')
