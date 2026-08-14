//! changelog 版本节提取（ADR-0016 release 重试通路）单元测试：节边界、
//! 头形态、前缀不匹配矩阵。

use vbumpp_core::changelog::extract_version_section;

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
