//! token 解析链：store → 环境变量 →（仅 github）gh CLI；
//! gitlab 在 store 级内先 host 作用域精确键、再 provider 级键回落

use std::collections::BTreeMap;

use vbumpp_core::release::{missing_token_message, resolve_token, Provider};

fn tokens(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
  pairs
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

fn no_env(_: &str) -> Option<String> {
  None
}

fn no_gh() -> Option<String> {
  None
}

#[test]
fn store_token_wins_over_env() {
  let env = |_: &str| Some("env-token".to_owned());
  let got = resolve_token(
    Provider::Github,
    None,
    &tokens(&[("github", "store-token")]),
    &env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("store-token"));
}

#[test]
fn empty_store_value_falls_through_to_env() {
  // JS `||` 语义：空字符串视为缺失
  let env = |key: &str| (key == "GH_TOKEN").then(|| "env-token".to_owned());
  let got = resolve_token(
    Provider::Github,
    None,
    &tokens(&[("github", "")]),
    &env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("env-token"));
}

#[test]
fn github_env_chain_order_and_fixed_spelling() {
  // GH_TOKEN 优先；GITHUB_TOKEN 次之（GITHOB_TOKEN 已移除——不再被读取）
  let both = |key: &str| match key {
    "GH_TOKEN" => Some("gh-token".to_owned()),
    "GITHUB_TOKEN" => Some("github-token".to_owned()),
    "GITHOB_TOKEN" => Some("typo-token".to_owned()),
    _ => None,
  };
  let got = resolve_token(Provider::Github, None, &BTreeMap::new(), &both, &no_gh);
  assert_eq!(got.as_deref(), Some("gh-token"));

  let only_github = |key: &str| match key {
    "GITHUB_TOKEN" => Some("github-token".to_owned()),
    "GITHOB_TOKEN" => Some("typo-token".to_owned()),
    _ => None,
  };
  let got = resolve_token(
    Provider::Github,
    None,
    &BTreeMap::new(),
    &only_github,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("github-token"));

  let only_typo = |key: &str| (key == "GITHOB_TOKEN").then(|| "typo-token".to_owned());
  let got = resolve_token(Provider::Github, None, &BTreeMap::new(), &only_typo, &no_gh);
  assert_eq!(got, None, "拼错的 GITHOB_TOKEN 不再生效");
}

#[test]
fn github_gh_cli_fallback_is_trimmed() {
  let gh = || Some("cli-token\n".to_owned());
  let got = resolve_token(Provider::Github, None, &BTreeMap::new(), &no_env, &gh);
  assert_eq!(got.as_deref(), Some("cli-token"));
}

#[test]
fn other_providers_have_own_env_and_no_gh_cli() {
  let env = |key: &str| match key {
    "GITLAB_TOKEN" => Some("gl".to_owned()),
    "GITEE_TOKEN" => Some("ge".to_owned()),
    "GITCODE_TOKEN" => Some("gc".to_owned()),
    _ => None,
  };
  let gh_called = || panic!("非 github 不应触达 gh CLI");
  assert_eq!(
    resolve_token(Provider::Gitlab, None, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("gl")
  );
  assert_eq!(
    resolve_token(Provider::Gitee, None, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("ge")
  );
  assert_eq!(
    resolve_token(Provider::Gitcode, None, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("gc")
  );
}

#[test]
fn store_key_is_per_provider() {
  let got = resolve_token(
    Provider::Gitlab,
    None,
    &tokens(&[("gitlab", "stored")]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("stored"));
  assert_eq!(
    resolve_token(
      Provider::Gitee,
      None,
      &tokens(&[("gitlab", "stored")]),
      &no_env,
      &no_gh
    ),
    None
  );
}

// ---------------------------------------------------------------------------
// gitlab 四级链：host 作用域精确键 → provider 级键回落 → GITLAB_TOKEN → 报错
// ---------------------------------------------------------------------------

#[test]
fn gitlab_host_scoped_key_wins_over_provider_key() {
  let got = resolve_token(
    Provider::Gitlab,
    Some("https://gitlab-a.com"),
    &tokens(&[
      ("gitlab@https://gitlab-a.com", "scoped-token"),
      ("gitlab", "provider-token"),
    ]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("scoped-token"));
}

#[test]
fn gitlab_provider_key_fallback_when_no_scoped_key() {
  // 向后兼容硬要求：存量自建 GitLab 用户的 token 都在 provider 级键下
  let got = resolve_token(
    Provider::Gitlab,
    Some("https://gitlab-a.com"),
    &tokens(&[("gitlab", "provider-token")]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("provider-token"));
}

#[test]
fn gitlab_host_normalization_collides_with_set_side() {
  // 配置值带尾斜杠/大小写差异/缺 scheme 时与 `token set --host` 的输入归一相撞
  for host in [
    "gitlab-a.com",
    "https://gitlab-a.com/",
    "HTTPS://GitLab-A.com",
  ] {
    let got = resolve_token(
      Provider::Gitlab,
      Some(host),
      &tokens(&[("gitlab@https://gitlab-a.com", "scoped-token")]),
      &no_env,
      &no_gh,
    );
    assert_eq!(got.as_deref(), Some("scoped-token"), "host={host}");
  }
}

#[test]
fn gitlab_scoped_key_miss_falls_to_env() {
  let env = |key: &str| (key == "GITLAB_TOKEN").then(|| "env-token".to_owned());
  let got = resolve_token(
    Provider::Gitlab,
    Some("https://gitlab-a.com"),
    &BTreeMap::new(),
    &env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("env-token"));
}

#[test]
fn gitlab_empty_scoped_value_falls_through() {
  // 空串按 JS `||` 语义视为缺失——继续回落 provider 级键
  let got = resolve_token(
    Provider::Gitlab,
    Some("https://gitlab-a.com"),
    &tokens(&[
      ("gitlab@https://gitlab-a.com", ""),
      ("gitlab", "provider-token"),
    ]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("provider-token"));
}

#[test]
fn gitlab_unnormalizable_host_skips_scoped_lookup() {
  // host 无法规范化（非 http(s) scheme）：跳过精确键查找照旧回落——非法
  // host 由 API 层报错，token 链不抢先变更既有行为
  let got = resolve_token(
    Provider::Gitlab,
    Some("ftp://gitlab-a.com"),
    &tokens(&[("gitlab", "provider-token")]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("provider-token"));
}

#[test]
fn non_gitlab_providers_ignore_host() {
  // host 参数仅 gitlab 消费（其他 provider 没有 host 配置通路）
  let got = resolve_token(
    Provider::Github,
    Some("https://ghe.example.com"),
    &tokens(&[
      ("github@https://ghe.example.com", "scoped"),
      ("github", "provider-token"),
    ]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("provider-token"));
}

#[test]
fn missing_message_carries_host_guidance_for_gitlab() {
  assert_eq!(
    missing_token_message(Provider::Gitlab, Some("https://gitlab-a.com")),
    "no Gitlab token detected for https://gitlab-a.com; \
     run vbumpp token set gitlab --host https://gitlab-a.com to add one"
  );
  // host 缺省（非 gitlab 通路）保持原文案
  assert_eq!(
    missing_token_message(Provider::Gitee, None),
    "no Gitee token detected; run vbumpp token set gitee to add one"
  );
}
