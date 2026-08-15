# Repo Scripts

本仓维护用命令行脚本的统一形态：一律 TypeScript，由 node 原生直跑（type stripping），不经编译或转译步骤。适用于 `scripts/`、`website/scripts/`、napi 冒烟（`napi/bumpp-core/smoke.ts`）、crates fixture 生成器（`crates/vbumpp-core/tests/fixtures/*-gen.ts`）。决策与权衡见 `docs/adr/0038-repo-scripts-node-native-ts.md`。

## Scope boundary

判别标准是「维护者或 CI 以命令行调用的脚本」。两类不适用，保持 `.js`：

- **发布物**：`npm/*/bin/` 薄壳随包上架、在用户不可控的 node 环境执行（ADR-0016 的 argv 透传薄壳）。
- **生成物**：napi loader 是 napi-rs 官方生成物（ADR-0033），工具产出而非手写维护面。

工具链消费的配置文件不属于命令行脚本（`website/postcss.config.mjs` 迁移时按共识保留 `.mjs`）；TS 配置入口（`vitest.config.ts`、`website/next.config.ts`）受下述语法与 tsconfig 约束覆盖，但同样不是脚本。

另有一例存量 shell 编排脚本：`website/scripts/capture-home-demo-cast.sh`——ADR-0036 cast 采集的 BSD `script(1)` pty 驱动层，会话录制形态由录制工具决定，保持 `.sh`；其伴生转换脚本（`raw-to-cast.ts` 等）已 TS 化、循本规范。

## Runtime

- `node scripts/foo.ts` 直接执行，零转译器、零编译产物、零 watcher；pnpm scripts、ci.yml、hk 的调用形态完全一致——本地怎么跑，CI 就怎么跑。
- 脚本一律 ESM 形态，所在包的 `package.json` 必须声明 `"type": "module"`（根、`napi/bumpp-core`、`npm/*`、`website` 均已声明）——缺了声明，node 直跑会先按 CJS 解析、失败后按 ESM 重解析并打出 `MODULE_TYPELESS_PACKAGE_JSON` 警告。
- 依赖 node 原生 type stripping（Node ≥22.18 起无 flag 可用）；版本下限由 mise 的 `node = "lts"` 保证——只涨不跌，不钉具体版本号。

## Type checking

type stripping 只擦除类型、不做类型检查。补位：根 `tsc --noEmit`（根 tsconfig 即脚本专用检查配置：`nodenext` / `strict` / `noEmit` / `allowImportingTsExtensions` / `erasableSyntaxOnly` / `types: ["node"]`），挂 hk pre-commit 秒级档、与 cargo fmt 并列（ADR-0031）。CI 不重复设类型检查腿——脚本在 CI 里被真实执行，node 直跑即最终形态。

## Syntax constraints

- `erasableSyntaxOnly` 是硬约束：禁 enum、namespace、参数属性（constructor parameter properties）等一切不可擦除语法——保证 tsc 可检查的语法 ⊆ node 可直跑的语法，永远不会被迫引入转译器。
- 文件间 import 必须写完整 `.ts` 后缀——node 按字面解析 specifier（不做 `.js`→`.ts` 替换），`allowImportingTsExtensions` 使 tsc 接受磁盘真实文件名形态；两端同一真相，不存在「tsc 放行、node 解析失败」的缝。

## tsconfig wiring

根 tsconfig 的 `include` 是脚本目录的显式清单：根 `scripts`、`napi/bumpp-core/smoke.ts`、`crates/vbumpp-core/tests/fixtures`、`vitest.config.ts`；`website` 整体 exclude，由 website 自身 tsconfig 覆盖（`**/*.ts` 天然含 `website/scripts/` 与 `next.config.ts`，`allowImportingTsExtensions` / `erasableSyntaxOnly` 两 flag 同款）。

新增脚本目录必须同步加进所在 tsconfig 的 `include`（根侧手工登记；website 侧 `**/*.ts` 已天然覆盖）——include 不进 = tsc 不查 = 类型门对它失效。
