# npm 渠道不支持 Intel Mac（darwin-x64）

`@vill-v/bumpp` 的 npm 安装依赖 `@vill-v/bumpp-core-*` 预编译平台包（ADR-0005）。本 ADR 明确：该平台包矩阵不纳入 darwin-x64，Intel Mac 不属于 npm 渠道的支持范围。

## Decisions

- npm 平台包矩阵固定为 5 targets：`darwin-arm64`、`linux-x64-gnu`、`linux-arm64-gnu`、`win32-x64-msvc`、`win32-arm64-msvc`，不含 darwin-x64。loader 在 Intel Mac 上抛出 "Supported platforms: …" 硬错误是既定行为，不为它加兜底分支。
- 不加 darwin-x64 平台包的理由：macOS 26 是最后一个支持 Intel 的系统版本，软件生态整体在收缩 x64；为一条正在退出的平台维护一条 CI 腿和一个 npm 包不值得。real trade-off 是 CI 时间与发布物数量换覆盖率——此处选择放弃覆盖。
- ADR-0025 中「darwin-x64 不入矩阵，回退编译兜底」仅适用于 cargo-binstall 渠道：binstall 在无匹配预编译产物时自动回退源码编译。npm 渠道没有这条兜底——`.node` 是原生共享库，arm64 构建产物不会加载进 x64 Node 进程，Rosetta 不救，且 npm 用户通常没有 Rust 工具链。两个渠道的「不支持」语义因此不同：binstall 是降级，npm 是硬失败。
- Intel Mac 用户的替代路径写进面向用户文档：改用 cargo 侧通路——免编译安装（cargo-binstall 渠道）在该平台自动回退源码编译，或直接 `cargo install vbumpp`；功能与 npm 版完全一致。

## Consequences

- Intel Mac 用户在 npm 安装阶段不报错（平台包是 optionalDependencies），首次运行时才遇 loader 硬错误；文档说明是唯一的软着陆点，必须保持准确。
- 若未来 Intel Mac 诉求回潮（企业存量设备等），重开评估的触发条件是出现真实用户反馈，而不是预防性补腿。
- 参考锚点：ADR-0025（binstall 渠道的矩阵与回退语义）、ADR-0005（napi 平台包分发机制）。
