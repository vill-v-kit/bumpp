//! 通用脚本执行（ADR-0011）：`bump.config.json` `scripts` 声明的 shell 命令，
//! 由 versionBump 流程在 preversion / version / postversion 三个时序槽位调用。
//! 本模块提供单条命令的执行原语；槽位编排见 bump.rs。

use std::path::Path;

use crate::exec::{run, ExecError};

/// 经系统 shell 执行命令串（Unix `sh -c`；Windows `cmd /d /s /c`——`/d /s`
/// 对齐 npm 默认：跳过注册表 AutoRun 钩子，避免用户机器环境污染发版）。
/// 非零退出即报错传播——配置声明的钩子失败时发版不得静默继续
/// （ADR-0011；对齐 ADR-0003 失败即报错精神，有意偏离上游 npm scripts
/// 未开 throwOnError 的不传播 parity）
pub fn run_script(cwd: &Path, command: &str) -> Result<(), ExecError> {
  #[cfg(windows)]
  let (shell, flags) = ("cmd", vec!["/d", "/s", "/c"]);
  #[cfg(not(windows))]
  let (shell, flags) = ("sh", vec!["-c"]);
  let args: Vec<String> = flags
    .into_iter()
    .map(str::to_string)
    .chain(std::iter::once(command.to_string()))
    .collect();
  run(shell, &args, cwd).map(|_| ())
}
