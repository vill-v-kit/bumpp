//! FuzzySelect 选项文本的 ANSI 不变量：dialoguer 渲染活动行
//! （选中样式 + fuzzy 高亮）会撕裂条目内嵌的转义序列，ESC 丢失后 `[1m`/`[0m`
//! 裸显——build_choices 产出的标题在强制开色下也不得含 ESC。
//! 独立 test target：console 颜色开关是进程级全局状态，与 info.rs 的文案断言隔离。

use vbumpp_core::prompt::build_choices;
use vbumpp_core::version::next_versions;

#[test]
fn choice_titles_never_embed_ansi_even_with_colors_forced() {
  dialoguer::console::set_colors_enabled(true);
  let next = next_versions("1.2.3", None, &[]).unwrap();
  let choices = build_choices("1.2.3", &next);
  dialoguer::console::set_colors_enabled(false);
  for (_, title) in &choices {
    assert!(
      !title.contains('\u{1b}'),
      "选项标题不得内嵌 ANSI：{title:?}"
    );
  }
  // 文案不因去样式而变（与非开色模式的断言一致）
  assert_eq!(choices[0].1, "        major 2.0.0");
  assert_eq!(choices[9].1, "       custom ...");
}
