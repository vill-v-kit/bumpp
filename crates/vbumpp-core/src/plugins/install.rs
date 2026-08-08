//! install 能力位：各生态 install 适配（ADR-0007）。
//! 触发编排（链走查 + 零生态命中回退 JavaScript）见插件底座（`plugins.rs`）。

pub mod cargo;
pub mod javascript;
