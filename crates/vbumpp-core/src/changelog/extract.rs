//! changelog 版本节提取：`vbumpp release` 重试通路的 body 来源——
//! 从 changelog 文件全文定位指定版本的 `## ` 节。匹配形状锚定生成侧
//! （`markdown.rs`：`## ` 头 → compare 链接 → 类型分组），`###`/`####`
//! 子节不终止节范围。

/// 提取指定版本的 changelog 节：从 `## v{version}` 或 `## {version}` 头行起，
/// 至下一个 `## ` 头（不含）或文件尾，两端空白 trim（对齐生成侧 body 的
/// `join("\n").trim()` 形状）。找不到该版本返回 None。
///
/// `version` 为裸版本号（不含 `v` 前缀——调用侧归一化）；两种头形态都匹配
/// 是因为 tagBody 模板可定制去前缀（`changelog.templates.tagBody`）。
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
