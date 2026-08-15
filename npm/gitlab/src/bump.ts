import { bumpVersion as coreBumpVersion } from '@vill-v/bumpp-core'
import type { BumpVersion, Config } from '@vill-v/bumpp'

/**
 * 更新版本并创建 Gitlab release（编排与 release 全在 Rust Core，ADR-0014；
 * 自建实例经配置 `gitlab.host` 段）
 */
export const bumpVersion = (option: Config = {}): Promise<BumpVersion> =>
  coreBumpVersion(option, 'gitlab')
