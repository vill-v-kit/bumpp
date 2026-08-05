#![deny(clippy::all)]

//! 原生 `vbumpp` 二进制（ADR-0019）：与 npm bin 共享同一 `run_from_argv` 的
//! 第二个薄壳前端——纯 Rust、零 napi 依赖。argv 语法（含 `--provider` flag
//! 与 `release` 子命令）的唯一归属在 Core 的 cli 模块，本壳仅透传 argv 并
//! 以返回码退出。

fn main() {
  let argv: Vec<String> = std::env::args().skip(1).collect();
  std::process::exit(vbumpp_core::cli::run_from_argv(&argv, None));
}
