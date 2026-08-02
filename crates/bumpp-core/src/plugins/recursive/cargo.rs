//! Cargo 生态清单 basename 常量（recursive 能力位，ADR-0010）。

/// 按清单处理的 basename：取磁盘惯例名（大写开头）——glob 模式与探测读取在
/// 大小写敏感文件系统（Linux）上才能命中真实文件；matches 识别仍为小写比较
pub(crate) const MANIFEST_BASENAMES: [&str; 1] = ["Cargo.toml"];
