//! token 解析链（ADR-0014）：store → 环境变量 →（仅 github）gh CLI

use std::collections::BTreeMap;

use vbumpp_core::release::{resolve_token, Provider};

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
  let got = resolve_token(Provider::Github, &tokens(&[("github", "")]), &env, &no_gh);
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
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &both, &no_gh);
  assert_eq!(got.as_deref(), Some("gh-token"));

  let only_github = |key: &str| match key {
    "GITHUB_TOKEN" => Some("github-token".to_owned()),
    "GITHOB_TOKEN" => Some("typo-token".to_owned()),
    _ => None,
  };
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &only_github, &no_gh);
  assert_eq!(got.as_deref(), Some("github-token"));

  let only_typo = |key: &str| (key == "GITHOB_TOKEN").then(|| "typo-token".to_owned());
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &only_typo, &no_gh);
  assert_eq!(got, None, "拼错的 GITHOB_TOKEN 不再生效");
}

#[test]
fn github_gh_cli_fallback_is_trimmed() {
  let gh = || Some("cli-token\n".to_owned());
  let got = resolve_token(Provider::Github, &BTreeMap::new(), &no_env, &gh);
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
    resolve_token(Provider::Gitlab, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("gl")
  );
  assert_eq!(
    resolve_token(Provider::Gitee, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("ge")
  );
  assert_eq!(
    resolve_token(Provider::Gitcode, &BTreeMap::new(), &env, &gh_called).as_deref(),
    Some("gc")
  );
}

#[test]
fn store_key_is_per_provider() {
  let got = resolve_token(
    Provider::Gitlab,
    &tokens(&[("gitlab", "stored")]),
    &no_env,
    &no_gh,
  );
  assert_eq!(got.as_deref(), Some("stored"));
  assert_eq!(
    resolve_token(
      Provider::Gitee,
      &tokens(&[("gitlab", "stored")]),
      &no_env,
      &no_gh
    ),
    None
  );
}
