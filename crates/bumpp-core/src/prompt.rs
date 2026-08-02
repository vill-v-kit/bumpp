//! 交互 prompt（Rust 渲染）：对齐上游 bumpp v11 `promptForNewVersion`。
//!
//! 选项集与文案逐一对齐（PADDING = 13、padStart 右对齐；customVersion 已随本重写移除，
//! 无 "from config"）。
//! 注意：选项文本一律不内嵌 ANSI 样式——dialoguer FuzzySelect 渲染活动行
//! （选中样式 + fuzzy 高亮）会撕裂条目内已有的转义序列，ESC 丢失后 `[1m`/`[0m`
//! 裸显（COL-30）。prompt 标题行的样式不受该路径影响，可正常使用。

use dialoguer::console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Input};
use semver::Version;

use crate::info::InfoError;
use crate::version::{NextVersions, ReleaseType};

/// 上游选项标签的 padStart 宽度
const PADDING: usize = 13;

/// 构建 prompt 选项 (value, title) 列表——纯函数，顺序与文案对齐上游（padStart 右对齐）
pub fn build_choices(current_version: &str, next: &NextVersions) -> Vec<(String, String)> {
  let entry = |label: &str, value: &str, version: &str| {
    (value.to_string(), format!("{label:>PADDING$} {version}"))
  };
  vec![
    entry("major", "major", &next.major),
    entry("minor", "minor", &next.minor),
    entry("patch", "patch", &next.patch),
    entry("next", "next", &next.next),
    entry("conventional", "conventional", &next.conventional),
    entry("pre-patch", "prepatch", &next.prepatch),
    entry("pre-minor", "preminor", &next.preminor),
    entry("pre-major", "premajor", &next.premajor),
    entry("as-is", "none", current_version),
    (
      "custom".to_string(),
      format!("{:>width$}", "custom ...", width = PADDING + 4),
    ),
  ]
}

/// 上游 `promptForNewVersion`：autocomplete 选择 + custom 二次输入。
/// 返回 (state.release, newVersion)——custom/next/conventional/none 时 release 为 None
pub fn prompt_new_version(
  current_version: &str,
  next: &NextVersions,
) -> Result<(Option<String>, String), InfoError> {
  let choices = build_choices(current_version, next);
  let titles: Vec<&str> = choices.iter().map(|(_, t)| t.as_str()).collect();
  let initial = choices
    .iter()
    .position(|(v, _)| v == "next")
    .expect("next 选项恒存在");

  let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
    .with_prompt(format!(
      "Current version {}",
      style(current_version).green()
    ))
    .items(&titles)
    .default(initial)
    .interact()
    .map_err(|e| InfoError::Prompt {
      message: format!("交互选择失败：{e}"),
    })?;

  let value = choices[selection].0.as_str();
  match value {
    "none" => Ok((None, current_version.to_string())),
    "custom" => {
      let input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter the new version number:")
        .default(current_version.to_string())
        .validate_with(|v: &String| {
          // 上游 valid(custom)：严格校验原样输入（含空白即拒绝）
          if Version::parse(v).is_ok() {
            Ok(())
          } else {
            Err("That's not a valid version number")
          }
        })
        .interact_text()
        .map_err(|e| InfoError::Prompt {
          message: format!("读取自定义版本失败：{e}"),
        })?;
      // 上游 clean(answers.custom)：校验已过，解析即格式化
      let cleaned = Version::parse(input.trim())
        .map(|v| v.to_string())
        .map_err(|_| InfoError::InvalidVersion {
          message: format!("无效的版本号：{input}"),
        })?;
      Ok((None, cleaned))
    }
    "next" => Ok((None, next.next.clone())),
    "conventional" => Ok((None, next.conventional.clone())),
    other => {
      // major / minor / patch / pre*：上游 state.release 记录选择值
      let release = ReleaseType::parse(other).expect("选项值集合固定");
      Ok((Some(other.to_string()), next.get(release).to_string()))
    }
  }
}
