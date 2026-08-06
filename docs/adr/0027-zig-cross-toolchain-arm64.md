# linux-arm64 交叉链换 zig（淘汰 gcc 4.8.5），TLS 回 rustls

`--use-napi-cross` 的 `@napi-rs/cross-toolchain` 捆绑 gcc 4.8.5（2015 年，官方 FAQ 与 cross-toolchain issue #4 确认），v6.0.0 发版三连炸的总根因：ring 与 aws-lc-sys 皆不可编，被迫临时切 native-tls + vendored openssl（ADR-0024）。本 ADR 把 aarch64-unknown-linux-gnu 腿换成 **zig 交叉链（cargo-zigbuild）** 并**撤销 ADR-0024 的 TLS 偏离，回到 ADR-0014 的 ureq + rustls**。COL-63 落地。

## Decisions

- **arm64 腿改直呼 `cargo zigbuild --target aarch64-unknown-linux-gnu.2.17`**，不走 napi CLI：napi CLI 的 target 校验只收合法 rust 三元组，**glibc 后缀被拒**（`--target aarch64-unknown-linux-gnu.2.17` 在 cargo metadata 阶段即报 "did you mean aarch64-unknown-linux-gnu?"）。产物 `libbumpp_core_napi.so` 由 CI 手动 `cp` 成 `bumpp-core.linux-arm64-gnu.node`——命名是 loader 的唯一契约，napi CLI 在 `--no-js` 下也只做这步改名。其余四腿仍走 `napi build`。
- **glibc 下限 pin 2.17，并在 CI 内断言**：后缀是必需项而非装饰——**裸 `--target aarch64-unknown-linux-gnu` 经 zig 会把下限抬到 2.30**（实测符号表出现 GLIBC_2.18/2.25/2.28/2.29/2.30）。矩阵里 pin 值单点维护（`glibc: '2.17'`），同时供 target 后缀与断言取用；断言判据是 `objdump -T` 里**最高**被引用版本 ≤ pin（不是全等——引用低于 pin 的版本合法，`GLIBC_PRIVATE` 无版本号不参与比较），回归即红。理由：平台包是公开分发物，静默抬下限等于砍掉一批老发行版用户。
- **TLS 回 rustls（ring 后端，ureq 3.x 默认）**：`ureq` features 从 `["json","gzip","native-tls","vendored"]` 收回 `["json","gzip","rustls"]`，openssl / openssl-src 出依赖图。ADR-0024 随之作废。
- **zig 与 cargo-zigbuild 写进 `mise.toml` 的 `[tools]`，但用 `os = ["linux"]` 限定**：由既有的 `jdx/mise-action` 步骤安装（带 token，版本与校验和落 `mise.lock`），zigbuild 腿直接 `cargo zigbuild`。`os` 限定让另四条腿（macOS/Windows）与本地开发不必白下载一套 zig——这条链的用途本身只在 Linux 主机上成立。macOS 上临时复现 arm64 产物走显式 spec 现取（`mise exec zig@… github:rust-cross/cargo-zigbuild@… -- cargo zigbuild …`），命令记在 `mise.toml` 注释里。
- **arm64 smoke test 进 CI 但不进 publish 前置**：`test-bindings-arm64-smoke` job 经 `docker/setup-qemu-action` 在 arm64 容器里 `require()` 产物并断言四个导出面。不列入 `publish-*` 的 `needs`——交叉腿实测是额外信心，napi-rs 生态（含 oxc）均不测交叉腿，不让它有权拦发版。

## Considered Options

- **`TARGET_CC=clang TARGET_CXX=clang++` + `--use-napi-cross`（官方 workaround 1）**：一行改动、glibc 保持 2.17，但只换 C 编译器，**链接器仍是上古 ld.bfd**（`unsupported GNU PROPERTY_TYPE (5)` 警告不消）。原计划作为诊断第一步验证「TLS 能否回 rustls」，因 zig 链一次实测即全指标达标（编译 38s + glibc 2.17 + 运行期 HTTPS 通），该实验无追加信息量，跳过。
- **`ubuntu-24.04-arm` 原生 runner**：工具链全现代且 arm 腿可原生跑测试，但**无缓解时 glibc 下限 2.39**——要守 2.17 仍得叠 zigbuild 或老 sysroot 容器，复杂度与 zig 方案趋同却多一台 runner 的形态差异。降为备选（zig 链失效时启用）。
- **升级 `@napi-rs/cross-toolchain`**：FAQ 未给已修版本，issue #4 未闭，时间表不可控——淘汰，仅留监控。
- **cross-rs（`--use-cross`）**：napi CLI 自己标注 "not recommended, prefer `--cross-compile` or `--use-napi-cross`"；需手工装 + 容器引擎，默认 aarch64 镜像 glibc 锁不到 2.17 且 gcc 版本不受控，最终仍要维护自定义镜像——比 zig 多一倍动件解同一问题，拒绝。
- **zig / cargo-zigbuild 不入 `[tools]`，CI 内 `mise exec <spec> -- cargo zigbuild` 现取**：省掉另四腿与本地开发的 zig 下载，一度是首选形态并已写进本 ADR 初稿。**实测否掉**：`mise exec` 解析 `github:` / `ubi:` 后端要打 GitHub API，`run` 步骤里没有 `GITHUB_TOKEN`，撞限流即 job 红（本地开发期已实撞一次 403）。构建可靠性优先于省一次下载，改为进 `[tools]` 走 mise-action 的带 token 安装。
- **oxc 式「macOS native-tls、其余 rustls」**：oxc 的 napi 产物不含 HTTP/TLS 栈（ureq 只在 `publish = false` 的 `tasks/common`，从不进交叉图），该模式对本仓（napi 产物必须带 TLS）不适用。oxc 的有效参考点是发布矩阵组织与 musl 腿的 zig 方案。

## Consequences

- **本地实测证据链**（macOS arm64 主机，2026-08-07）：`cargo zigbuild --target aarch64-unknown-linux-gnu.2.17` 编译 38s 通过（含 rustls 0.23 / ring）；产物 ELF aarch64、`DT_NEEDED` 仅 libm/libpthread/libc/libdl、GLIBC 引用仅 2.17；arm64 容器（qemu，node:22-bookworm）内 `require()` 成功、`release` 子命令对 api.github.com 完成真实 rustls 握手并干净返回 `[401] Bad credentials`——ring 在 aarch64 的编译期与运行期风险双双关闭。
- **残留一条良性警告**：`warning: linker stderr: ignoring deprecated linker optimization setting '1'`。来源是 **rustc 对 gnu 目标默认传的 `-Wl,-O1`**（非本仓传入），zig 的 lld 已废弃 `-O<n>`（`-O2` 同样告警）。不用 `-A linker-messages` 吞掉——那会连未来真警告一起静音。与旧链的 `unsupported GNU PROPERTY_TYPE (5)` 不同级：后者是链接器读不懂现代目标文件元数据，前者只是忽略一个过时开关。
- **arm64 腿失去 napi CLI 的 dts 产出**，故 artifact 上传分两路：四腿传 `.node + index.d.ts`（`if-no-files-found: error`），zigbuild 腿只传 `.node`。`index.d.ts` 由其余四腿供给 publish-npm，注入逻辑不变。
- **构建时间下降**：不再逐次源码编译 openssl。
- **未来 musl 平台包（ADR-0026）可复用这条 zig 链**——oxc 的 musl 腿正是同款方案；注意 musl 的 cdylib 需 `-C target-feature=-crt-static`（napi CLI 自动加，直呼 zigbuild 时需手动补）。
