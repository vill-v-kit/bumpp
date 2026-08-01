//! 纯 Rust 实现的 bumpp 版本引擎，语义全量对齐上游 antfu/bumpp v11。
//! 不依赖 napi——Node 绑定层在 `npm/bumpp-core`。

pub mod commits;
pub mod config;
pub mod exec;
pub mod files;
pub mod git;
pub mod info;
mod jsonc;
pub mod progress;
pub mod prompt;
pub mod scripts;
pub mod version;
