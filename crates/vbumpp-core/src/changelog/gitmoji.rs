//! gitmoji 数据表：convert-gitmoji@0.1.5 的 74 条官方映射原样内建。
//!
//! 原实现 `convert(content, true)`：全文本大小写不敏感替换 `:code:` 为
//! emoji + 尾随空格。原实现以未转义键名拼正则（`+` 等字符沦为量词），
//! 属潜在 bug；本实现按字面量转义匹配——对真实输入行为一致，病理输入
//! （如 `:heavy_plusss_sign:`）不再误中。

use std::sync::LazyLock;

use regex::Regex;

/// 官方 gitmoji 映射（`:code:` → unicode emoji），序与原包一致
static GITMOJIS: [(&str, &str); 74] = [
  (":art:", "🎨"),
  (":zap:", "⚡️"),
  (":fire:", "🔥"),
  (":bug:", "🐛"),
  (":ambulance:", "🚑️"),
  (":sparkles:", "✨"),
  (":memo:", "📝"),
  (":rocket:", "🚀"),
  (":lipstick:", "💄"),
  (":tada:", "🎉"),
  (":white_check_mark:", "✅"),
  (":lock:", "🔒️"),
  (":closed_lock_with_key:", "🔐"),
  (":bookmark:", "🔖"),
  (":rotating_light:", "🚨"),
  (":construction:", "🚧"),
  (":green_heart:", "💚"),
  (":arrow_down:", "⬇️"),
  (":arrow_up:", "⬆️"),
  (":pushpin:", "📌"),
  (":construction_worker:", "👷"),
  (":chart_with_upwards_trend:", "📈"),
  (":recycle:", "♻️"),
  (":heavy_plus_sign:", "➕"),
  (":heavy_minus_sign:", "➖"),
  (":wrench:", "🔧"),
  (":hammer:", "🔨"),
  (":globe_with_meridians:", "🌐"),
  (":pencil2:", "✏️"),
  (":pencil:", "✏️"),
  (":poop:", "💩"),
  (":rewind:", "⏪️"),
  (":twisted_rightwards_arrows:", "🔀"),
  (":package:", "📦️"),
  (":alien:", "👽️"),
  (":truck:", "🚚"),
  (":page_facing_up:", "📄"),
  (":boom:", "💥"),
  (":bento:", "🍱"),
  (":wheelchair:", "♿️"),
  (":bulb:", "💡"),
  (":beers:", "🍻"),
  (":speech_balloon:", "💬"),
  (":card_file_box:", "🗃️"),
  (":loud_sound:", "🔊"),
  (":mute:", "🔇"),
  (":busts_in_silhouette:", "👥"),
  (":children_crossing:", "🚸"),
  (":building_construction:", "🏗️"),
  (":iphone:", "📱"),
  (":clown_face:", "🤡"),
  (":egg:", "🥚"),
  (":see_no_evil:", "🙈"),
  (":camera_flash:", "📸"),
  (":alembic:", "⚗️"),
  (":mag:", "🔍️"),
  (":label:", "🏷️"),
  (":seedling:", "🌱"),
  (":triangular_flag_on_post:", "🚩"),
  (":goal_net:", "🥅"),
  (":dizzy:", "💫"),
  (":wastebasket:", "🗑️"),
  (":passport_control:", "🛂"),
  (":adhesive_bandage:", "🩹"),
  (":monocle_face:", "🧐"),
  (":coffin:", "⚰️"),
  (":test_tube:", "🧪"),
  (":necktie:", "👔"),
  (":stethoscope:", "🩺"),
  (":bricks:", "🧱"),
  (":technologist:", "🧑‍💻"),
  (":money_with_wings:", "💸"),
  (":thread:", "🧵"),
  (":safety_vest:", "🦺"),
];

/// 全键名 alternation（字面量转义，大小写不敏感）
static GITMOJI_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(?i):art:|:zap:|:fire:|:bug:|:ambulance:|:sparkles:|:memo:|:rocket:|:lipstick:|:tada:|:white_check_mark:|:lock:|:closed_lock_with_key:|:bookmark:|:rotating_light:|:construction:|:green_heart:|:arrow_down:|:arrow_up:|:pushpin:|:construction_worker:|:chart_with_upwards_trend:|:recycle:|:heavy_plus_sign:|:heavy_minus_sign:|:wrench:|:hammer:|:globe_with_meridians:|:pencil2:|:pencil:|:poop:|:rewind:|:twisted_rightwards_arrows:|:package:|:alien:|:truck:|:page_facing_up:|:boom:|:bento:|:wheelchair:|:bulb:|:beers:|:speech_balloon:|:card_file_box:|:loud_sound:|:mute:|:busts_in_silhouette:|:children_crossing:|:building_construction:|:iphone:|:clown_face:|:egg:|:see_no_evil:|:camera_flash:|:alembic:|:mag:|:label:|:seedling:|:triangular_flag_on_post:|:goal_net:|:dizzy:|:wastebasket:|:passport_control:|:adhesive_bandage:|:monocle_face:|:coffin:|:test_tube:|:necktie:|:stethoscope:|:bricks:|:technologist:|:money_with_wings:|:thread:|:safety_vest:").unwrap()
});

/// convert-gitmoji `convert(content, true)`：`:code:` → emoji + 尾随空格
pub fn convert_gitmoji(content: &str) -> String {
  GITMOJI_RE
    .replace_all(content, |caps: &regex::Captures| {
      let code = caps.get(0).unwrap().as_str().to_ascii_lowercase();
      let emoji = GITMOJIS
        .iter()
        .find(|(k, _)| *k == code)
        .map(|(_, v)| v)
        .expect("a regex-matched key is always in the table");
      format!("{emoji} ")
    })
    .into_owned()
}
