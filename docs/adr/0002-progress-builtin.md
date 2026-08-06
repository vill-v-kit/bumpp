# progress 内置到 Rust，拆除 JS 进度面

`versionBump` 的进度回调从 JS 入参改为 Rust 内置：执行到各步骤时由 Rust 直接打印进度（仿 consola 样式：绿色 `✔` success、蓝色 `ℹ` info，输出到 stdout），CLI 用户体验不变。`versionBump` 的 `progress` 入参、napi 层 ThreadsafeFunction 机制、`ProgressEvent` 枚举导出与 `VersionBumpProgress` 负载类型随之全部拆除；Rust 内部事件流保留（作为内置打印的数据源与 cargo 测试的观测点）。

## Decisions

- **内置打印样式**：复刻 npm/bump 原 JS progress 的输出（`✔ Updated x to 2.0.0`、`ℹ x did not need to be updated`、`Git commit`、`Git tag`、`✔ Git push`、`✔ Npm run x`），颜色经 console crate（TTY 自动降级为纯文本）；输出通道为 stdout（与 dialoguer prompt、printSummary 一致）。
- **打印逻辑抽纯函数**：事件 + 状态 → 字符串的格式化函数独立可测；打印只是薄壳副作用。
- **对外删除**：`versionBump(options)` 不再接受 `progress`；`ProgressEvent` 不再导出到 Node 层；`versionBump` 返回值（`VersionBumpResults`）保留不变。
- **对内保留**：Rust 内部 ProgressEvent 事件流与 cargo 层事件断言不变。
- **测试分层调整**：事件顺序与负载由 cargo 内部闭包断言（现状）；打印样式由 cargo 单测覆盖格式化函数；vitest 缝隙只断言外部行为（文件内容、git 状态、results），不再测事件序列。

## Considered Options

- **保留可选 progress 回调**（不传时内置打印）：与"入参删除"目标相悖，且目前无任何消费方需要程序化订阅——拒绝，需要时再以新 API 加回。
- **静默不打印**：CLI 失去过程感——拒绝。

## Consequences

- `npm/bump/src/bump.ts` 的 JS progress 函数删除，`versionBump` 调用不再传 progress。
- 对上游 bumpp API 形状形成新偏差：上游 `progress` 为公开入参，本实现内置——本仓是唯一消费方，无兼容性损失。
- 进度行中的路径打印格式后经 ADR-0023 修订（显示层统一为：cwd 内相对、cwd 外绝对、一律 POSIX）；本 ADR 的「复刻上游」范围自此不含路径形态。
