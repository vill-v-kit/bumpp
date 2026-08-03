//! 纯 Rust 实现的 bumpp 版本引擎，语义全量对齐上游 antfu/bumpp v11。
//! 不依赖 napi——Node 绑定层在 `napi/bumpp-core`。

pub mod bump;
pub mod changelog;
pub mod commits;
pub mod config;
pub mod exec;
pub mod git;
pub mod home;
pub mod info;
mod jsonc;
pub mod orchestrate;
pub mod plugins;
pub mod progress;
pub mod prompt;
pub mod release;
pub mod scripts;
pub mod token;
pub mod version;
