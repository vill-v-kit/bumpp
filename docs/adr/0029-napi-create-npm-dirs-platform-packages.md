# 平台包目录由 napi create-npm-dirs 生成，不提交进 git

5+2 个平台包目录（`napi/bumpp-core-<triple>/`）原为手写提交：每个目录一份 `package.json` + `LICENSE`，与 `napi/bumpp-core/package.json` 的字段（author、homepage、engines、publishConfig……）靠人肉保持同步，drift 风险随平台数线性增长。本 ADR 把平台包目录切换为 `napi create-npm-dirs` 从 `napi.targets` 生成、gitignore 不跟踪，并相应修订 ADR-0021 的 npm 发布流（注入步骤改为生成步骤）。

## Decisions

- `napi/bumpp-core/package.json` 的 `napi.targets`（7 条 rust triple）是支持平台的单一真相源。真相源链：`napi.targets` → `napi create-npm-dirs` 生成平台目录 → `optionalDependencies`（`workspace:*`）→ loader 从 optionalDependencies 动态推导支持清单。loader.test.ts 不再维护硬编码平台数组。
- 平台包目录固定为 `napi/<platformArchABI>/`（如 `napi/linux-x64-musl`）——目录名由 create-npm-dirs 按 target 派生，不可配置；包名仍是 `@vill-v/bumpp-core-<triple>` 不变。生成物为 `package.json` + `README.md`；`os`/`cpu`/`libc` 字段由 triple 自动派生（gnu → `libc: ["glibc"]`，musl → `libc: ["musl"]`）。
- 平台包目录不提交进 git（`.gitignore` 整体忽略）。可行性经 pnpm 源码 + 复现双验证：`optionalDependencies` 里的 `workspace:*` 在目标 workspace 包不在磁盘时 silent skip（exit 0，不报错、不 registry fallback）。fresh clone `pnpm install --frozen-lockfile` 照常通过；本地开发 loader 走本包根目录的本地 `.node` fallback。提交的 lockfile 记录 7 个平台包 importer 与 `link:../<triple>`——fresh clone 上 link 目标暂缺无害（pnpm 不校验 link 目标存在性；目录生成后链接自动生效）。
- 取 `create-npm-dirs` + `napi artifacts` 两件，不引入 `napi pre-publish` 全套。pre-publish 被否决：其版本/optionalDependencies 同步非原子、不清理 stale optionalDeps、`.node` 归属仅按文件名后缀信任；本仓库版本号唯一维护点是根 workspace 版本（-r 整树收集锁步），不需要它的版本同步。发布仍走 `pnpm publish -r`（ADR-0021 的 skip-if-published 幂等守卫不变）。
- `napi artifacts` 要求 `napi.targets` 全部 target 的 `.node` 齐备（缺一即硬失败），只能在汇聚全腿产物的 publish-npm job 执行；各 build 腿职责不变（构建 + 上传 artifact）。调用经 `scripts/collect-artifacts.mjs`（`pnpm collect:artifacts`）包装：与 `create-npm-dirs.mjs` 同一组 bin 路径与旗标知识，并把 `index.d.ts` 归位 core。
- create-npm-dirs 不产物 LICENSE；`scripts/create-npm-dirs.mjs`（`pnpm create:npm-dirs`）包装生成 + 根 LICENSE 逐目录同步，与 check-licenses.mjs 的校验口径一致。CI 的 test job 与 publish-npm job 都经此包装生成。
- 生成的 `package.json` 经 `pick` 机制保留 `publishConfig.access`/`engines`/`license`/`author`/`homepage`；`type: "commonjs"` 被丢弃但无害（`main` 指向 `.node`，npm 默认 CJS 解析，行为等价）；`repository.directory` 继承 `napi/bumpp-core`（平台目录无源码，反而更对）。字段集 drift 风险由此消除。
- 生成的平台目录命中根 `Cargo.toml` 的 `napi/*` glob 但无 `Cargo.toml`，仍在 `exclude` 清单显式排除（目录名从 `napi/bumpp-core-<triple>` 变为 `napi/<triple>`，清单同步改写）。

## Consequences

- 新增/移除平台的触点收敛为一组小清单：`napi.targets`（真相源）+ CI build matrix + `optionalDependencies` 一条 `workspace:*` + `.gitignore` 与根 `Cargo.toml` `exclude` 各一条目录名（两者受目录枚举格式约束，无法进一步收敛）；loader、loader.test.ts 与生成物字段随之自动收敛。
- publish-npm job 的注入步骤从「手写 for 循环 cp .node 进 tracked 目录」变为「create:npm-dirs 生成目录 + napi artifacts 按名分发」；pack/publint/publish 的 glob 覆盖不变。
- 本地执行 `pnpm create:npm-dirs` 后，平台目录存在于磁盘、frozen-lockfile 语义不受影响（lockfile 已含全部平台包记录）；该状态与 fresh clone 的差异只在于目录是否在磁盘。
- `pnpm pack`/`publish` 的 `workspace:*` 版本改写要求平台包已安装（符号链接已建）：frozen install 预建的 link 在目录生成后自动生效，无需二次 install。
- 首发新增平台包（如 musl ×2）仍按 ADR-0021 决策④的一次性 token 首发 → trusted publisher 配置 → 撤 token 闭环执行。
- 参考锚点：ADR-0021（发布流与 OIDC）、ADR-0025（musl 平台包与 zig 交叉链）、ADR-0005（平台包归 `napi/` 的受众判别）。
