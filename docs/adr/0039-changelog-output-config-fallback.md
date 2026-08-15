# changelog.output 统一回落配置：release 读配置、CLI 默认值不再覆盖（cac 默认值 parity 退场）

配置类型审计（ADR-0037 研讨）的附带发现（COL-101）：`vbumpp release` 子命令定位 changelog 文件只认 `--output` flag（默认 `CHANGELOG.md`），不读配置文件的 `changelog.output`——用户改了输出路径后补发报 cannot read；bump 主命令则总把 CLI 默认 `changelog.output: "CHANGELOG.md"` 注入 overrides（旧 cli.ts 的 cac 默认值语义原样收编），配置文件里的 `changelog.output` 对 CLI 恒不生效。两怪癖同根：CLI 默认值被当成显式值对待。决定统一为「`-o` flag > 配置 `changelog.output` > 内建默认」单一优先级链。

## Decisions

- **`-o` 是唯一的 CLI 注入点**：bump 的 overrides 仅在 argv 显式给出 `-o/--output` 时注入 `changelog` 段；未给时四层配置合并照常生效，内建默认 `CHANGELOG.md` 兜底。cac 默认值恒传的 parity 退场——它是 JS 时代 cac 总把选项默认值物化进解析结果的副产品；手写解析器（ADR-0016）能区分「显式给出」与「默认」，继续维持 parity 就是让用户的配置静默失效，违背「配置写对即生效」的承诺（ADR-0003 失败即报错、ADR-0013 统一配置的精神）。
- **release 与 bump 同路解析 changelog 路径**：release 子命令在 `-o` 未给出时经与 bump 同一条配置解析路径（`read_document` 全局 ← 项目文档合并 + changelog 段解析）取 `changelog.output`；release 无自定义配置路径机制（无 overrides 入口），文档层固定按探测加载，其余环节与 bump 完全同源。release 是 bump 的重试通路——bump 写哪份 changelog，release 就该读哪份；同一配置同一路径，不存在两套定位语义。
- **配置不可解析即报错**：release 读配置失败（文件层键名/类型校验、changelog 段 pre-pass）与 bump 一样即时报错 exit 1，不静默回落默认——两通路对同一份配置的前置语义保持一致。

## Alternatives considered

- **保留 cac 默认值 parity（被否决）**：与上游 JS 行为逐字节一致，存量用户零变化。但被否决：该 parity 本身就是上游怪癖（配置的 `changelog.output` 从未被 CLI 兑现）；本仓已是全 Rust 一等 CLI，cac 物化默认值的机制原因不复存在，维持它只兑现怪癖不兑现兼容；且 bump 写默认路径而 release 也读默认路径时用户虽无感，一旦配置了输出路径两个怪癖同时显形——修复一侧不修另一侧反而更不一致。
- **release 读配置失败时静默回落默认（被否决）**：补发通路对坏配置更宽容。但被否决：同一份配置 bump 报错、release 放行，两通路语义分裂；且「找不到 changelog」的报错会发生在下一步（cannot read），不如配置报错直指根因。失败即报错是本仓定例（ADR-0003）。

## Consequences

- 用户可见行为变化：配置了 `changelog.output` 的用户，bump 此前恒写 `CHANGELOG.md`，此后写配置路径——对他们而言是配置兑现而非破坏；未配置的用户行为不变（内建默认兜底）。随 minor 发版在 release note 说明。
- release 子命令自此有配置加载依赖：坏配置会拦住补发（即时报错）；`-o` flag 仍是无条件旁路（flag 给出时完全不读配置）。
- ADR-0016 的 release 重试条目（「从 `--output` 指定的 changelog 文件（默认 `CHANGELOG.md`）提取」）随本决策更新为含配置回落的表述。
