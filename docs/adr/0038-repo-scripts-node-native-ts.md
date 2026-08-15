# 仓库脚本统一 TypeScript 由 node 原生直跑：type stripping + 根 tsc 类型门补位

迁移前，仓库维护脚本是 15 个 `.mjs`（发布守卫、漂移校验、artifact 收集、演示采集、许可证检查等），复杂度已到需要类型的量级——发布链脚本一旦写错是发版当场爆炸，而 `.mjs` 只有 JSDoc 注释类型、无编译期把关。决定全量迁 TypeScript，由 node 原生 type stripping 直跑，不引入 tsx / ts-node / 编译步骤；类型检查以根 `tsc --noEmit` 挂 hk pre-commit 补位（迁移已落地，本 ADR 落档规范）。

## Decisions

- **全量 TypeScript、node 直跑**：仓库脚本一律 `.ts`，`node scripts/foo.ts` 原生执行——零转译器、零编译产物、零 watcher。本地 pnpm scripts、ci.yml、hk 的调用形态完全一致，只有一条执行路径，不存在「检查的」与「执行的」漂移。
- **Node 下限 ≥22.18，由 mise `lts` 保证**：type stripping 自 22.18 起无 flag 可用；`mise.toml` 钉 `node = "lts"`，下限只涨不跌、不钉具体版本号。仓库脚本只在本仓维护者环境与 CI 执行，环境受控——不需要对任意 node 版本普适，这是与发布物的本质区别。
- **类型检查由根 tsc `--noEmit` 补位，挂 hk pre-commit**：type stripping 只擦除类型、不做类型检查。根 tsconfig 即脚本专用检查配置（`nodenext` / `strict` / `noEmit` / `allowImportingTsExtensions` / `erasableSyntaxOnly` / `types: ["node"]`），tsc 步骤入 hk pre-commit 秒级档、与 cargo fmt 并列（ADR-0031 的门内新增一步）。CI 不重复设类型检查腿——脚本在 CI 里被真实执行，node 直跑即最终形态。
- **`erasableSyntaxOnly` 硬约束**：禁 enum、namespace、参数属性等一切不可擦除语法。它保证「tsc 可检查的语法 ⊆ node 可直跑的语法」——类型标注永远可机械擦除，永远不会被迫引入转译器，这是整个方案的自洽前提而非风格偏好。
- **import 必须带完整 `.ts` 后缀**：node 按字面解析 specifier（不做 `.js`→`.ts` 替换），`allowImportingTsExtensions` 使 tsc 接受磁盘真实文件名形态；两端同一真相，堵住「tsc 放行、node 解析失败」的缝。
- **tsconfig include 是脚本目录显式清单**：根 include 手工登记根 `scripts`、napi 冒烟、crates fixture 生成器、`vitest.config.ts`；website 整体从根 exclude，由自身 tsconfig 覆盖（`**/*.ts` 天然含其脚本与 `next.config.ts`，两 flag 同款）。新增脚本目录必须同步登记所在 tsconfig 的 include——include 不进则类型门对它静默失效。
- **排除边界**：两类保持 `.js` 不适用本决策——① 发布物 `npm/*/bin/` 薄壳：随包上架、在用户不可控的 node 环境执行（ADR-0016 的 argv 透传薄壳）；② 生成物 napi loader：napi-rs 官方生成物（ADR-0033），工具产出而非手写维护面。工具链消费的配置文件（`website/postcss.config.mjs`）按迁移共识保留原扩展名，不属命令行脚本范畴；存量 pty 采集编排脚本 `website/scripts/capture-home-demo-cast.sh` 保持 shell——`script(1)` 会话录制形态由录制工具决定（ADR-0036），其伴生转换脚本已 TS 化。

## Alternatives considered

- **维持 `.mjs`（被否决）**：对任意 node 版本普适、零工具链假设。这是本决策的核心权衡的另一端：放弃 `.mjs` 的版本普适性，换取类型安全与可维护性——但仓库脚本的执行环境受控（mise + CI），普适性无处兑现；而发布链、漂移校验这类脚本的类型错误代价持续走高，JSDoc 注释类型约束弱、维护成本不低于 TS 本身。
- **tsx / ts-node 转译执行（被否决）**：引入额外 devDependency 与第二套执行语义（transform），「tsc 检查的」与「实际执行的」存在漂移空间；node 原生直跑只有一条路径，且 node 已内置同能力，依赖纯冗余。
- **编译步骤（tsc emit / 打包器产出 dist，被否决）**：产生源码与产物的双份真相——提交产物则每次改动同步两处，gitignore 产物则 fresh clone 与 CI 需先构建才能跑脚本；脚本无分发需求，编译纯增成本。
- **JSDoc 注释类型（被否决）**：保住 `.mjs` 的普适性又能获得部分检查；但无法表达 `erasableSyntaxOnly` 这类「语法子集」硬约束、注解噪音大，迁移工作量与全量 TS 相当而收益打折。

## Consequences

- 贡献者与 CI 的 node 版本由此约束为 ≥22.18（mise `lts` 保证）；用户侧不受影响——发布物（`npm/*/bin/` 薄壳）不依赖 type stripping，用户 node 版本不可控的边界由排除边界守住。
- hk pre-commit 是仓库脚本类型检查的唯一挂载点；新增脚本目录须登记 tsconfig include（根侧手工，website 侧天然覆盖），规范细则落 `docs/agents/scripts.md`。
- `erasableSyntaxOnly` 永久排除 enum、namespace 等语法；未来若某脚本确需这些能力，意味着它已超出「可擦除脚本」范畴，应重新评估方案（改写为可擦除形态、或挪入 Rust 侧）而非绕过约束。
- `website` 依赖 Next.js 工具链消费自身配置，`postcss.config.mjs` 类工具探测文件保持原扩展名；这不构成对「一律 TS」的例外，因为它们不是命令行脚本。
