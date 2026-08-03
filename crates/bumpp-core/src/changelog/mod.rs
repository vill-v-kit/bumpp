//! changelog 域（ADR-0012）：changelogen 使用面的 Rust 重写。
//! 编排与对外 API 收于此根部；能力子目录：配置段解析（`config`）。
//! markdown 生成（`markdown`）与 gitmoji 数据表（`gitmoji`）随后续工单落地。

pub mod config;
