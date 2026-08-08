# cargo-binstall 预编译 CLI 渠道：7 targets、musl 与可恢复发布

`vbumpp` 已进入 crates.io，但 `cargo install vbumpp` 仍需源码编译。本 ADR 将 GitHub Release 预编译产物和 `cargo binstall vbumpp` 立为与 npm 并列的一等安装渠道；无匹配平台时保留 binstall 的源码编译回退。

## Decisions

- 发布矩阵固定为 7 targets：现有 napi 平台对应的 `aarch64-apple-darwin`、`x86_64-unknown-linux-gnu`、`aarch64-unknown-linux-gnu`、`x86_64-pc-windows-msvc`、`aarch64-pc-windows-msvc`，以及 `x86_64-unknown-linux-musl`、`aarch64-unknown-linux-musl`。glibc 系统同时可探测 gnu 和 musl；Intel macOS 不纳入，回退源码编译。
- 产物命名为 `vbumpp-{target}.tar.gz`，归档顶层目录为 `vbumpp-{target}/`，内含 `vbumpp`。该命名和 `v{version}` tag 直接命中 binstall 默认探测模板，因此 `crates/vbumpp/Cargo.toml` 不添加 `[package.metadata.binstall]`。每个归档另附 `.sha256`，供用户手工校验；暂不引入 minisign。
- CI 由独立 `build-cli` job 负责 7-target matrix，不与 napi build 混合。darwin/Windows 原生 cargo build；linux arm64-gnu 与两个 musl target 使用 mise 安装的 Zig 和 `cargo-zigbuild`。musl 共享 zig 交叉链，构建 cdylib 或其他动态产物时必须注意 `-C target-feature=-crt-static` 的加载语义。
- linux arm64-gnu 必须使用 `aarch64-unknown-linux-gnu.2.17` target spec，并在 CI 断言产物引用的最高 GLIBC 版本不超过 2.17；裸 target 不接受，因为会把下限抬至现代 glibc。该约束保护公开分发对老发行版的兼容性。
- napi 的 musl 平台包（ADR-0025 原 0026 的合并内容）与 CLI 共用 Zig 经验，但仍属于 napi build matrix 的独立平台腿；平台包为 `@vill-v/bumpp-core-linux-x64-musl` 与 `@vill-v/bumpp-core-linux-arm64-musl`，主包通过 optionalDependencies 分发，声明 `os/cpu/libc`。npm 计数由 11 增至 13；旧包管理器即使双装 gnu/musl，loader 仍兜底选择正确二进制。
- npm musl 采用 `napi build --cross-compile`，其内部使用 cargo-zigbuild 并为 musl 处理 `-C target-feature=-crt-static`；不要把 `--use-napi-cross` 当作 musl 或 arm64-gnu 的构建链。Alpine smoke 至少在 x64 容器中完成安装、加载和 `--version`/导出面验证；arm64 经 QEMU 尽力验证，失败须明确记录为已知限制。
- HTTP/TLS 固定使用 `ureq` + rustls（ring）；zig 提供现代 C/链接工具链，避免旧 gcc 4.8.5 无法编译 ring/aws-lc。TLS 选型与 provider 边界见 ADR-0014。
- `build-cli` 在 crates 发布之后运行，归档和 checksum 缺失即硬失败；使用 `gh release upload --clobber` 追加到 bump 流程已创建的 GitHub Release。构建或上传失败不影响已完成的 npm/crates 发布，GitHub Re-run failed jobs 可安全重跑并收敛。
- 只从下一版本 tag 起提供产物，不回填 v6.0.0 及以前的 Release。

## Consequences

- 产物文件名、归档目录、tag 路径和 glibc 2.17 是 binstall 的隐式兼容契约；改变任一项必须同步补 metadata 或验证默认探测行为。
- GitHub Release 从 changelog 承载体扩展为 CLI 二进制分发载体；发布主链路创建 Release 的职责不变，build-cli 只追加 assets。
- musl 平台包使 npm 在 Alpine 上从硬失败变为原生安装；编程式 napi 用户不能以 binstall CLI 替代它。
- arm64 交叉构建不经 napi CLI 时可能不产出 dts，npm 发布继续由其他 napi 腿提供共享 `index.d.ts`；未来扩展 zig 腿必须保留这一供给关系。
