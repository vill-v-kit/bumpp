//! changelog 版本节提取（ADR-0019）：`vbumpp release` 重试通路的 body 来源——
//! 从 changelog 文件全文定位指定版本的 `## ` 节。匹配形状锚定生成侧
//! （`markdown.rs`：`## ` 头 → compare 链接 → 类型分组），`###`/`####`
//! 子节不终止节范围。

/// 提取指定版本的 changelog 节：从 `## v{version}` 或 `## {version}` 头行起，
/// 至下一个 `## ` 头（不含）或文件尾，两端空白 trim（对齐生成侧 body 的
/// `join("\n").trim()` 形状）。找不到该版本返回 None。
///
/// `version` 为裸版本号（不含 `v` 前缀——调用侧归一化）；两种头形态都匹配
/// 是因为 tagBody 模板可定制去前缀（`changelog.templates.tagBody`，ADR-0013）。
pub fn extract_version_section(content: &str, version: &str) -> Option<String> {
  let heading_v = format!("## v{version}");
  let heading_plain = format!("## {version}");
  let lines: Vec<&str> = content.lines().collect();
  let start = lines.iter().position(|line| {
    let line = line.trim_end();
    line == heading_v || line == heading_plain
  })?;
  let end = lines[start + 1..]
    .iter()
    .position(|line| line.starts_with("## "))
    .map(|offset| start + 1 + offset)
    .unwrap_or(lines.len());
  Some(lines[start..end].join("\n").trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  const SAMPLE: &str = "# Changelog\n\
     \n\
     ## v5.1.0\n\
     \n\
     [compare changes](https://example.com/compare/v5.0.1...v5.1.0)\n\
     \n\
     ### 🚀 Features\n\
     \n\
     - add something (abc1234)\n\
     \n\
     #### 🚨 Breaking\n\
     \n\
     - drop old api (def5678)\n\
     \n\
     ## v5.0.1\n\
     \n\
     ### 🐛 Fixes\n\
     \n\
     - fix thing (9990000)\n";

  #[test]
  fn extracts_first_section_stopping_at_next_heading() {
    let section = extract_version_section(SAMPLE, "5.1.0").unwrap();
    assert_eq!(
      section,
      "## v5.1.0\n\n[compare changes](https://example.com/compare/v5.0.1...v5.1.0)\n\n### 🚀 Features\n\n- add something (abc1234)\n\n#### 🚨 Breaking\n\n- drop old api (def5678)"
    );
  }

  #[test]
  fn extracts_last_section_to_eof() {
    let section = extract_version_section(SAMPLE, "5.0.1").unwrap();
    assert_eq!(
      section,
      "## v5.0.1\n\n### 🐛 Fixes\n\n- fix thing (9990000)"
    );
  }

  #[test]
  fn matches_heading_without_v_prefix() {
    let content = "# Changelog\n\n## 5.1.0\n\n### Fixes\n\n- x (aaaaaaa)\n";
    let section = extract_version_section(content, "5.1.0").unwrap();
    assert!(section.starts_with("## 5.1.0"));
  }

  #[test]
  fn missing_version_returns_none() {
    assert_eq!(extract_version_section(SAMPLE, "9.9.9"), None);
  }

  #[test]
  fn exact_match_does_not_bleed_into_prerelease() {
    // `## v5.1.0-beta.1` 不是 `5.1.0` 的节（精确相等，非前缀匹配）
    let content = "## v5.1.0-beta.1\n\n- pre (aaaaaaa)\n";
    assert_eq!(extract_version_section(content, "5.1.0"), None);
    assert!(extract_version_section(content, "5.1.0-beta.1").is_some());
  }
}
