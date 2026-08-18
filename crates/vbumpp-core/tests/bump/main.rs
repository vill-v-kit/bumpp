//! Bump 域测试镜像 src/bump/：执行全链路用例在 `bump`，dry-run 计划
//!用例在 `dry_run`。共享工具经 main 统一声明，子模块走 `super::`。

#[path = "../common.rs"]
mod common;

mod bump;
mod dry_run;
