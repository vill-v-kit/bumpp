import { defineConfig } from '@vill-v/bumpp-gitee'

export default defineConfig({
  bumpp: {
    // 发版时同步更新版本的 Cargo.toml（ADR-0003：显式列明，不做 recursive 收集；
    // 各 manifest 的 [package].version 与根 Cargo.lock 对应条目由插件链定向同步）
    files: ['crates/bumpp-core/Cargo.toml', 'napi/bumpp-core/Cargo.toml'],
  },
  changelog: {
    excludeAuthors: [],
  },
})
