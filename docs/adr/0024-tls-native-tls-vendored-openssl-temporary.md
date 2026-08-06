# TLS 栈暂偏离 rustls 至 native-tls + vendored openssl（COL-63 落地后回评撤销）

> **Status: superseded by ADR-0027（2026-08-07）。** 本 ADR 记的临时偏离已按预定路径撤销——zig 交叉链落地后 TLS 回 rustls（ring），ADR-0014 的选型恢复生效。保留本文用于解释 v6.0.0 期间那段 native-tls + vendored openssl 的历史，勿据此配置 TLS。

ADR-0014 决策 HTTP 栈为 ureq 3.x + rustls（明确「无 OpenSSL」）。v6.0.0 tag CI 两连炸后，TLS 栈临时切到 native-tls + vendored openssl，当时只记录在 `crates/vbumpp-core/Cargo.toml` 注释里——本 ADR 补记这次对 ADR-0014 的偏离及其缘由，并钉死回评路径。**ADR-0014 的其余决策不受影响，仅 TLS 选型一条被本 ADR 暂时取代。**

## 背景（v6.0.0 三连炸）

napi cross-toolchain 1.0.3 的 aarch64 交叉工具链是 gcc 4.8.5（2015 年，官方确认根因：napi-rs FAQ「rustls / aws-lc-sys fails with --use-napi-cross on aarch64」→ cross-toolchain issue #4）：

1. **ring 0.17**（rustls 默认后端）的 aarch64 预生成汇编要求 clang 专有预定义 `__ARM_ARCH`，上古 gcc 无 → 炸；
2. 换 **aws-lc-rs**：现代 C 在上古 gcc 上遍地开花（缺 `stdatomic.h`、不识 `-march=armv8.4-a+sha3`、glibc 头无 `AT_HWCAP2`）→ 再炸；
3. 切 **native-tls + vendored openssl**：openssl 是移植面最广的 C 栈，上古 gcc 可编 → 绿。

## Decisions

- **TLS 暂用 native-tls + vendored openssl**：linux 两腿经 `openssl-src` 源码构建，macOS/Windows 走系统 TLS（SecureTransport/Schannel），无额外 C 构建。
- **这是临时偏离而非选型变更**：rustls 被拒绝的不是其本身，而是其后端（ring/aws-lc）在上古交叉链上的可编译性。COL-63（linux-arm64 构建链现代化）落地、工具链换现代 clang（zig 链或原生 arm runner）后，**回 rustls（ring 后端，ureq 3.x 默认），移除 `native-tls`+`vendored` features 与 openssl-src 依赖图，本 ADR 标注撤销，ADR-0014 的 TLS 选型恢复生效**。
- **回评触发条件**：COL-63 验收项「linux-arm64-gnu 腿无 linker 警告」达成时，同 PR 或紧随 PR 完成 TLS 回切，避免临时栈固化。

## Considered Options

- **留在 rustls + ring / aws-lc-rs**：两个后端在上述工具链上均不可编（炸点实例见上）——发版当下不可行，拒绝。
- **升级 napi cross-toolchain / 换交叉链后再回 rustls**：即 COL-63 路线，是治本但属调研+迁移体量，v6.0.0 发版阻塞等不起——故先临时偏离，回评路径如上。
- **oxc 式「macOS native-tls、其余 rustls」**：oxc 的 napi 产物不含 HTTP/TLS 栈，其 ureq 仅在原生构建的开发任务里——该模式对本仓（napi 产物必须带 TLS）不适用，拒绝照搬（COL-63 修订已记此事实核查）。

## Consequences

- 现实代价（回切 rustls 前持续支付）：linux 两腿每次构建付 openssl 源码编译时间；aarch64 腿遗留 linker 警告（`ld.bfd: unsupported GNU PROPERTY_TYPE (5)`——上古 ld.bfd 连 rustlib 的现代 GNU property 都读不了）；依赖图多出 `openssl` / `openssl-src`。
- 若 COL-63 长期不落地，本临时栈有固化风险——届时重读本 ADR 与 COL-63 验收，勿把 `native-tls` 当成既定选型。
- 参考锚点：COL-63（构建链调研，含 zig 首选终态与 glibc ≥2.17 下限验收）、napi-rs FAQ 对应条目、`crates/vbumpp-core/Cargo.toml` 的 ureq feature 注释。
