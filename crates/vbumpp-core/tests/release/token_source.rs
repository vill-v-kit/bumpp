//! token 来源签名：`resolve_token_sourced` 返回「token + 来源」，
//! 供 --dry-run 的来源报告消费；`resolve_token` 保持裸 token 兼容形态不变。

use std::collections::BTreeMap;

use vbumpp_core::release::{resolve_token_sourced, Provider, TokenSource};

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
fn store_hit_reports_store_source() {
  let env = |_: &str| Some("env-token".to_owned());
  let got = resolve_token_sourced(
    Provider::Github,
    None,
    &tokens(&[("github", "store-token")]),
    &env,
    &no_gh,
  )
  .unwrap();
  assert_eq!(got.token, "store-token");
  assert_eq!(got.source, TokenSource::Store);
}

#[test]
fn env_hit_reports_exact_variable_name() {
  // GH_TOKEN 优先时来源为 GH_TOKEN（链序与裸 token 形态一致）
  let both = |key: &str| match key {
    "GH_TOKEN" => Some("gh-token".to_owned()),
    "GITHUB_TOKEN" => Some("github-token".to_owned()),
    _ => None,
  };
  let got = resolve_token_sourced(Provider::Github, None, &BTreeMap::new(), &both, &no_gh).unwrap();
  assert_eq!(got.token, "gh-token");
  assert_eq!(got.source, TokenSource::Env("GH_TOKEN"));

  // 次级变量命中时来源为 GITHUB_TOKEN
  let only_github = |key: &str| (key == "GITHUB_TOKEN").then(|| "github-token".to_owned());
  let got = resolve_token_sourced(
    Provider::Github,
    None,
    &BTreeMap::new(),
    &only_github,
    &no_gh,
  )
  .unwrap();
  assert_eq!(got.token, "github-token");
  assert_eq!(got.source, TokenSource::Env("GITHUB_TOKEN"));

  // 其余三家各自的变量名
  let env = |key: &str| (key == "GITLAB_TOKEN").then(|| "gl".to_owned());
  let got = resolve_token_sourced(Provider::Gitlab, None, &BTreeMap::new(), &env, &no_gh).unwrap();
  assert_eq!(got.source, TokenSource::Env("GITLAB_TOKEN"));
}

#[test]
fn gh_cli_fallback_reports_gh_source() {
  let gh = || Some("cli-token\n".to_owned());
  let got = resolve_token_sourced(Provider::Github, None, &BTreeMap::new(), &no_env, &gh).unwrap();
  assert_eq!(got.token, "cli-token", "trim 语义不变");
  assert_eq!(got.source, TokenSource::GhCli);
}

#[test]
fn empty_store_value_falls_through_and_reports_real_source() {
  // 空串按 JS `||` 语义视为缺失——来源须为实际命中的下一级，不得谎报 store
  let env = |key: &str| (key == "GH_TOKEN").then(|| "env-token".to_owned());
  let got = resolve_token_sourced(
    Provider::Github,
    None,
    &tokens(&[("github", "")]),
    &env,
    &no_gh,
  )
  .unwrap();
  assert_eq!(got.token, "env-token");
  assert_eq!(got.source, TokenSource::Env("GH_TOKEN"));
}

#[test]
fn source_descriptions_are_user_readable() {
  assert_eq!(TokenSource::Store.describe(), "token store");
  assert_eq!(
    TokenSource::Env("GH_TOKEN").describe(),
    "environment variable GH_TOKEN"
  );
  assert_eq!(TokenSource::GhCli.describe(), "gh CLI (`gh auth token`)");
}

#[test]
fn gitlab_scoped_hit_reports_store_source() {
  // host 作用域精确键命中：来源同为 token store（四级链的 store 级内细分）
  let got = resolve_token_sourced(
    Provider::Gitlab,
    Some("https://gitlab-a.com"),
    &tokens(&[
      ("gitlab@https://gitlab-a.com", "scoped-token"),
      ("gitlab", "provider-token"),
    ]),
    &no_env,
    &no_gh,
  )
  .unwrap();
  assert_eq!(got.token, "scoped-token");
  assert_eq!(got.source, TokenSource::Store);
}
