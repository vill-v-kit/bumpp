//! JavaScript 生态清单 basename 常量（recursive 能力位，ADR-0007）：
//! 上游 switch 列表，小写比较；recursive 收集模式与默认清单（ADR-0007）的单一事实源。

/// 按 manifest 处理的 basename（上游 switch 列表，小写比较）
pub(crate) const MANIFEST_BASENAMES: [&str; 8] = [
  "package.json",
  "package-lock.json",
  "bower.json",
  "component.json",
  "jsr.json",
  "jsr.jsonc",
  "deno.json",
  "deno.jsonc",
];
