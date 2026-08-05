//! recursive 能力矩阵：链上清单模式表聚合（ADR-0003 opt-in；ADR-0010 底座化）。

#[test]
fn recursive_manifest_globs_cover_all_ecosystems() {
  // 链上模式表（ADR-0003 opt-in）：node 8 种 manifest + Cargo.toml；
  // Text 兜底无清单概念不贡献。basename 取磁盘惯例名（Cargo.toml 大写开头）——
  // 大小写敏感文件系统（Linux）上 glob 才能命中真实文件
  assert_eq!(
    vbumpp_core::plugins::recursive_manifest_globs(),
    vec![
      "**/package.json",
      "**/package-lock.json",
      "**/bower.json",
      "**/component.json",
      "**/jsr.json",
      "**/jsr.jsonc",
      "**/deno.json",
      "**/deno.jsonc",
      "**/Cargo.toml",
    ]
  );
}

#[test]
fn default_file_patterns_are_root_level_chain_union() {
  // ADR-0009：files 为空时的默认清单 = 链上 manifest basenames 根级并集
  // （glob 展开使不存在的文件自然消失，无需运行时生态探测）
  assert_eq!(
    vbumpp_core::plugins::default_file_patterns(false),
    vec![
      "package.json",
      "package-lock.json",
      "bower.json",
      "component.json",
      "jsr.json",
      "jsr.jsonc",
      "deno.json",
      "deno.jsonc",
      "Cargo.toml",
    ]
  );
}

#[test]
fn default_file_patterns_recursive_upgrades_to_tree_globs() {
  // ADR-0009：recursive 默认清单 = 同一份 basename 表的 `**/` 整树收集模式
  // （替代 bump.rs 原 `packages/**/package.json` 硬编码）
  assert_eq!(
    vbumpp_core::plugins::default_file_patterns(true),
    vbumpp_core::plugins::recursive_manifest_globs()
  );
}
