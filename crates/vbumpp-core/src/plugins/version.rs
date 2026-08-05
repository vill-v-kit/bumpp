//! 版本能力位：版本解析与保格式版本更新，每生态一文件（ADR-0010）。
//! text 为兜底通道（仅有更新能力）。各文件经插件类型的 trait 方法一行委托调用。

pub(crate) mod cargo;
pub(crate) mod node;
pub(crate) mod text;
