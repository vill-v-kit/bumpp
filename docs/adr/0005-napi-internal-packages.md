# napi/ 收纳内部机制包

`napi/` 按受众收纳 Rust↔Node 绑定及其二进制分发机制包；`npm/` 收纳用户直接安装使用的 npm 包。是否发布到 npm 不参与目录判定。

## Decisions

- 判别问句是“用户会直接 npm install 它吗？”：不会则放 `napi/`，会则放 `npm/`。
- `@vill-v/bumpp-core` 绑定本体及各平台 optionalDependencies 包属于内部机制包，均位于 `napi/`。它们发布到 npm 是 workspace 引用和按平台分发机制的要求，不代表用户入口。
- `@vill-v/bumpp` 与 provider 变体包是面向用户的安装入口，位于 `npm/`。
- 目录名保留 `napi/`，因为当前内部机制包均属于 napi 绑定或其平台二进制分发。
- 根 Cargo workspace 的 members/exclude、CI 产物路径和 package metadata 必须与该目录边界保持一致。

## Consequences

- 目录直接表达 npm 包受众，内部发布机制与用户入口不会混居。
- 新包先按受众归类；若未来出现不属于 napi 机制的内部 npm 包，再重新评估目录命名。
