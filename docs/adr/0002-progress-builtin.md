# Rust 内置进度输出

版本更新过程的进度由 Rust 内置格式化并直接打印，Node API 不再暴露进度回调或 `ProgressEvent` 类型。进度事件流仍作为 Rust 内部打印与测试的事实来源。

## Decisions

- 进度输出保持现有用户可见语义：更新、跳过、git commit/tag/push、脚本执行等步骤打印到 stdout；TTY 下使用颜色，非 TTY 自动降级为纯文本。
- 事件与状态到字符串的格式化是可单测的纯函数，打印本身是薄副作用层。
- `versionBump(options)` 不接受 `progress`；`VersionBumpResults` 及 Rust 内部事件顺序保留。
- 路径只在显示层格式化：cwd 内显示相对 POSIX 路径，cwd 外显示绝对 POSIX 路径；事件负载、存储路径和 API 返回值仍保持绝对原生路径。
- 进度之外的警告和含路径错误同样使用统一显示路径规则；用户输入回显和不含路径的 release 行不转换。

## Consequences

- JS progress 函数和 napi 进度面已移除，调用方只观察外部结果。
- 输出有意不完全复刻上游的绝对原生路径形态；同一显示规则覆盖进度、警告和路径错误。
- Rust 测试覆盖事件、格式化结果和外部文件/git 行为，Node 集成测试只断言外部结果。
