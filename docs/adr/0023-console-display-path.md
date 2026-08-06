# 控制台路径显示：cwd 内相对、cwd 外绝对、一律 POSIX

进度行、警告与含路径的错误信息此前直接打印绝对原生路径（`resolve(cwd, rel)` 的结果，刻意复刻上游 bumpp 的 `path.resolve` 行为，见 ADR-0002），与同一次运行里已是相对 POSIX 的收集列表、changelog 输出风格混杂；Windows 上还会打出 `\` 分隔符。本 ADR 确立统一的显示路径规则：打印到控制台的路径，cwd 之内打相对路径，cwd 之外打绝对路径，一律 POSIX 分隔符（`/`）。转换只发生在显示层——`updated_files`、napi `updatedFiles` 等 API 返回值保持绝对原生路径不变，上游 parity 面不受影响。

## Decisions

- **规则一句话**：打印的路径永远是 POSIX 分隔符；cwd 内打相对，cwd 外打绝对。`strip_prefix(cwd)` 失败即落绝对分支。
- **显示时转换**：存储与事件负载保持绝对原生路径（git pathspec、测试断言的确定性不变），打印前过一道纯函数转换，落在 ADR-0002 划出的「格式化纯函数 + 打印薄壳」分层内。
- **适用面**：进度行、untracked 警告、含路径的错误信息（config / changelog / 插件读写 / Cargo.lock 同步 / token 存储）。摘要的 `files` 回显是用户输入 echo，release 行不含路径，均不动。
- **锚点为 cwd**：不引入 git root 发现机制；CLI 的 `env.cwd` 覆盖语义不变，从子目录调用时打印的即「从这里看过去的路径」。
- **cwd 外典型成员**：token 存储与全局配置（home 目录）、`..` 逃逸的显式文件参数、任何 `strip_prefix` 失败的兜底——打绝对 POSIX 路径。

## Considered Options

- **存储即改**（事件/结果字符串直接存相对 POSIX）：全链单一表示、打印零转换，但 napi `updatedFiles` 返回值形状偏离上游——本仓对 parity 面的一贯方向是收缩砍掉而非改形状，拒绝。
- **git root 锚定**：从子目录调用时会打出 `../sub/package.json`，且要为非 git 目录定义降级行为，机制成本与显示收益不成比例，拒绝。
- **cwd 外路径 `..` 相对化**：`../../other/package.json` 可读性差且长度不稳定，拒绝。
- **cwd 外保持绝对原生**（不 POSIX 化）：Windows 上同一会话 `\` 与 `/` 混排，规则无法一句话陈述，拒绝。

## Consequences

- 对上游 bumpp 输出格式的又一次刻意偏离（继 ADR-0002 进度回调内置、ADR-0014/0016 API 收缩之后）；ADR-0002 的「复刻上游」范围自此不含路径形态。
- 存储与事件负载仍是绝对原生路径：`tests/plugins/main.rs:150` 与 `tests/bump.rs:381` 两处断言存储值的 POSIX 假设测试不受影响（仍仅在 unix CI 上成立）；Windows cargo test 腿不在本次范围，另行评估。
- 转换为字符串级操作（`strip_prefix` + `\`→`/`），与 `git.rs` `filter_tracked` 的既有先例同法；unix 文件名含 `\` 的极端情况会被误转换，接受（同先例）。
- CONTEXT.md 立「显示路径 (Display path)」术语，归「用户可见字符串」规则簇。
