//! token 子命令行为矩阵（镜像 src/cli/token.rs）：set / list / remove 三动作、
//! remove 交互矩阵与 flag 扫描 helper 的单元测试。

use std::path::PathBuf;

use tempfile::TempDir;
use vbumpp_core::cli::scan_token_args;
use vbumpp_core::token::{read_token_store_at, save_token_at, TokenError};

use super::{argv, run, run_full, run_remove, store_in};

// ---------------------------------------------------------------------------
// token 子命令
// ---------------------------------------------------------------------------

#[test]
fn token_bare_errors_usage() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token <action> [name]"), "{err}");
}

#[test]
fn token_unknown_action_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "peek"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(
    err.contains("unknown action: peek (expected set / list / remove)"),
    "{err}"
  );
}

#[test]
fn token_set_without_name_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token set <name>"), "{err}");
}

#[test]
fn token_set_dash_prefixed_name_errors_usage() {
  // token 子命令 flag 扫描：声明名单外的 `--x` 一律按未知 flag 报错（exit 1）
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set", "--output"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unknown option: --output"), "{err}");
}

#[test]
fn token_set_cancelled_warns_without_writing() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(None);
  let (out, _err, code) = run_full(
    &["token", "set", "github"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "取消不是失败，退出码");
  assert!(out.contains("entry canceled"), "{out}");
  assert!(
    read_token_store_at(&store).unwrap().is_empty(),
    "取消不落盘"
  );
}

#[test]
fn token_set_saves_and_reports() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("secret-token".to_string()));
  let (out, _err, code) = run_full(
    &["token", "set", "github"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("github token saved (encrypted)"), "{out}");
  assert_eq!(
    read_token_store_at(&store).unwrap()["github"],
    "secret-token",
    "明文经加密存储可回读"
  );
}

#[test]
fn token_set_prompt_error_surfaces() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| {
    Err(TokenError::Prompt {
      message: "token must not be empty".to_string(),
    })
  };
  let (_out, err, code) = run_full(
    &["token", "set", "github"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("token must not be empty"), "{err}");
}

#[test]
fn token_list_empty_store() {
  let dir = TempDir::new().unwrap();
  let (out, _err, code) = run(&["token", "list"], &store_in(&dir));
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no tokens configured"), "{out}");
}

#[test]
fn token_list_prints_all_names() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "github", "a").unwrap();
  save_token_at(&store, "gitee", "b").unwrap();
  let (out, _err, code) = run(&["token", "list"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitee"), "{out}");
  assert!(out.contains("github"), "{out}");
}

#[test]
fn token_path_ignores_provider() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let (out, _err, code) = run_full(&["token", "list"], Some("bogus"), &store, None, None);
  assert_eq!(code, 0, "provider 对 token 通路无影响，退出码");
  assert!(out.contains("no tokens configured"), "{out}");
}

// ---------------------------------------------------------------------------
// token set / list --host（host 作用域键）
// ---------------------------------------------------------------------------

#[test]
fn token_set_gitlab_with_host_saves_scoped_key() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("scoped-secret".to_string()));
  let (out, _err, code) = run_full(
    &["token", "set", "gitlab", "--host", "https://gitlab-a.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "退出码");
  assert!(
    out.contains("gitlab (https://gitlab-a.com) token saved (encrypted)"),
    "{out}"
  );
  assert_eq!(
    read_token_store_at(&store).unwrap()["gitlab@https://gitlab-a.com"],
    "scoped-secret",
    "存储内出现 host 作用域复合键"
  );
}

#[test]
fn token_set_host_prompt_names_target_host() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |name: &str| {
    assert_eq!(
      name, "gitlab (https://gitlab-a.com)",
      "prompt 文案指明目标 host"
    );
    Ok(Some("x".to_string()))
  };
  let (_out, _err, code) = run_full(
    &["token", "set", "gitlab", "--host", "gitlab-a.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "退出码");
}

#[test]
fn token_set_host_normalization_collapses_to_same_key() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("x".to_string()));
  for raw in [
    "gitlab-a.com",
    "https://gitlab-a.com/",
    "HTTPS://GitLab-A.com",
  ] {
    let (_out, _err, code) = run_full(
      &["token", "set", "gitlab", "--host", raw],
      None,
      &store,
      None,
      Some(&prompt),
    );
    assert_eq!(code, 0, "{raw} 退出码");
  }
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens.len(), 1, "三种写法归一到同一键，实际 {tokens:?}");
  assert!(tokens.contains_key("gitlab@https://gitlab-a.com"));
}

#[test]
fn token_set_host_equals_form_and_double_dash_separator() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let prompt = |_name: &str| Ok(Some("x".to_string()));
  // `--host=H` 等值形态
  let (_out, _err, code) = run_full(
    &["token", "set", "gitlab", "--host=https://gitlab-b.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "--host=H 退出码");
  // `--` 之后一律位置参数（`--host` 不再解析为 flag）
  let (_out, _err, code) = run_full(
    &["token", "set", "--", "gitee", "--host=https://gitlab-c.com"],
    None,
    &store,
    None,
    Some(&prompt),
  );
  assert_eq!(code, 0, "-- 分隔退出码");
  let tokens = read_token_store_at(&store).unwrap();
  assert!(
    tokens.contains_key("gitlab@https://gitlab-b.com"),
    "{tokens:?}"
  );
  assert!(
    tokens.contains_key("gitee"),
    "-- 后 --host 按位置参数忽略，gitee 落 provider 级键：{tokens:?}"
  );
}

#[test]
fn token_set_host_rejects_non_gitlab_providers() {
  for provider in ["github", "gitee", "gitcode"] {
    let dir = TempDir::new().unwrap();
    let store = store_in(&dir);
    let prompt = |_name: &str| Ok(Some("x".to_string()));
    let (_out, err, code) = run_full(
      &["token", "set", provider, "--host", "https://gitlab-a.com"],
      None,
      &store,
      None,
      Some(&prompt),
    );
    assert_eq!(code, 1, "{provider} 退出码");
    assert!(
      err.contains("--host is only supported for gitlab"),
      "{provider}: {err}"
    );
    assert!(
      read_token_store_at(&store).unwrap().is_empty(),
      "{provider} 拒绝路径不落盘"
    );
  }
}

#[test]
fn token_set_host_missing_value_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "set", "gitlab", "--host"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("option --host requires a value"), "{err}");
}

#[test]
fn token_set_host_invalid_value_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(
    &["token", "set", "gitlab", "--host", "https://"],
    &store_in(&dir),
  );
  assert_eq!(code, 1, "退出码");
  assert!(
    err.contains("invalid host: https:// (missing host name)"),
    "{err}"
  );
}

#[test]
fn token_list_shows_friendly_scoped_names() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-a.com", "scoped").unwrap();
  let (out, _err, code) = run(&["token", "list"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  assert!(
    out.lines().any(|line| line.ends_with("gitlab")),
    "provider 级键按原名显示：{out}"
  );
}

#[test]
fn token_list_host_filters_single_entry() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-a.com", "a").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-b.com", "b").unwrap();
  // 过滤值同样经规范化（无 scheme 写法可命中）
  let (out, _err, code) = run(&["token", "list", "--host", "gitlab-a.com"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  assert!(!out.contains("gitlab-b.com"), "{out}");
  assert!(
    !out.lines().any(|line| line.ends_with("gitlab")),
    "provider 级键被过滤：{out}"
  );
}

#[test]
fn token_list_host_filter_without_match_warns() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  let (out, _err, code) = run(&["token", "list", "--host", "gitlab-a.com"], &store);
  assert_eq!(code, 0, "未命中非失败，退出码");
  assert!(
    out.contains("no token found for host https://gitlab-a.com"),
    "{out}"
  );
}

#[test]
fn token_list_unknown_flag_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "list", "--wat"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unknown option: --wat"), "{err}");
}

#[test]
fn token_remove_without_name_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(&["token", "remove"], &store_in(&dir));
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token remove <name>"), "{err}");
}

#[test]
fn token_remove_absent_warns_and_keeps_store() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "x").unwrap();
  let (out, _err, code) = run(&["token", "remove", "github"], &store);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no token found for github"), "{out}");
  assert_eq!(
    read_token_store_at(&store).unwrap().len(),
    1,
    "误删不存在键不动存储"
  );
}

#[test]
fn token_remove_existing_succeeds() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "github", "x").unwrap();
  // --yes 跳过确认直删
  let (out, _err, code) = run_remove(&["token", "remove", "github", "--yes"], &store, None);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("github token removed"), "{out}");
  assert!(
    read_token_store_at(&store).unwrap().is_empty(),
    "删除后存储为空"
  );
}

// ---------------------------------------------------------------------------
// token remove 交互矩阵：四目标形态 × 执行修饰（--dry-run / --yes / 确认）
// ---------------------------------------------------------------------------

/// 矩阵用例共享的存储形态：provider 级键 + 两个 host 作用域键 + 别家键
fn seeded_store(dir: &TempDir) -> PathBuf {
  let store = store_in(dir);
  save_token_at(&store, "gitlab", "plain").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-a.com", "a").unwrap();
  save_token_at(&store, "gitlab@https://gitlab-b.com", "b").unwrap();
  save_token_at(&store, "gitee", "other").unwrap();
  store
}

#[test]
fn remove_provider_key_leaves_scoped_keys() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (_out, _err, code) = run_remove(&["token", "remove", "gitlab", "--yes"], &store, None);
  assert_eq!(code, 0, "退出码");
  let tokens = read_token_store_at(&store).unwrap();
  assert!(!tokens.contains_key("gitlab"), "provider 级键已删");
  assert!(
    tokens.contains_key("gitlab@https://gitlab-a.com")
      && tokens.contains_key("gitlab@https://gitlab-b.com"),
    "精确项语义不触碰 gitlab@* 键：{tokens:?}"
  );
  assert!(tokens.contains_key("gitee"));
}

#[test]
fn remove_host_scoped_key_only() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  // 过滤值带尾斜杠：经同一规范化函数与写入侧相撞
  let (_out, _err, code) = run_remove(
    &[
      "token",
      "remove",
      "gitlab",
      "--host",
      "https://gitlab-a.com/",
      "--yes",
    ],
    &store,
    None,
  );
  assert_eq!(code, 0, "退出码");
  let tokens = read_token_store_at(&store).unwrap();
  assert!(
    !tokens.contains_key("gitlab@https://gitlab-a.com"),
    "目标 host 键已删"
  );
  assert!(tokens.contains_key("gitlab"), "provider 级键不动");
  assert!(
    tokens.contains_key("gitlab@https://gitlab-b.com"),
    "其他 host 键不动"
  );
}

#[test]
fn remove_host_rejects_non_gitlab() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (_out, err, code) = run_remove(
    &[
      "token",
      "remove",
      "gitee",
      "--host",
      "https://gitlab-a.com",
      "--yes",
    ],
    &store,
    None,
  );
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("--host is only supported for gitlab"), "{err}");
  assert_eq!(
    read_token_store_at(&store).unwrap().len(),
    4,
    "拒绝路径不动存储"
  );
}

#[test]
fn remove_host_scoped_missing_warns() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (out, _err, code) = run_remove(
    &[
      "token",
      "remove",
      "gitlab",
      "--host",
      "https://gitlab-c.com",
      "--yes",
    ],
    &store,
    None,
  );
  assert_eq!(code, 0, "目标不存在非失败，退出码");
  assert!(
    out.contains("no token found for gitlab (https://gitlab-c.com)"),
    "{out}"
  );
  assert_eq!(read_token_store_at(&store).unwrap().len(), 4, "不动存储");
}

#[test]
fn remove_provider_all_partial_listing_without_provider_key() {
  // provider 级键不存在但 host 键在：--all 只删实际存在的条目并如实列清单
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab@https://gitlab-a.com", "a").unwrap();
  save_token_at(&store, "gitee", "other").unwrap();
  let (out, _err, code) = run_remove(
    &["token", "remove", "gitlab", "--all", "--yes"],
    &store,
    None,
  );
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  assert!(
    !out.lines().any(|line| line.ends_with(" gitlab")),
    "不存在的 provider 级键不入清单：{out}"
  );
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens.len(), 1, "{tokens:?}");
  assert!(tokens.contains_key("gitee"));
}

#[test]
fn remove_provider_all_clears_scoped_keys() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (out, _err, code) = run_remove(
    &["token", "remove", "gitlab", "--all", "--yes"],
    &store,
    None,
  );
  assert_eq!(code, 0, "退出码");
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens.len(), 1, "gitlab 全清、别家不动：{tokens:?}");
  assert!(tokens.contains_key("gitee"));
  // 如实列清单（友好形态）
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  assert!(out.contains("gitlab (https://gitlab-b.com)"), "{out}");
}

#[test]
fn remove_all_empties_whole_store() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (_out, _err, code) = run_remove(&["token", "remove", "--all", "--yes"], &store, None);
  assert_eq!(code, 0, "退出码");
  assert!(!store.exists(), "清空后存储文件删除（既有语义）");
}

#[test]
fn remove_all_on_empty_store_warns() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  let (out, _err, code) = run_remove(&["token", "remove", "--all", "--yes"], &store, None);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no tokens configured"), "{out}");
}

#[test]
fn remove_provider_all_without_match_warns() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "x").unwrap();
  let (out, _err, code) = run_remove(
    &["token", "remove", "gitlab", "--all", "--yes"],
    &store,
    None,
  );
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no token found for gitlab"), "{out}");
  assert_eq!(read_token_store_at(&store).unwrap().len(), 1, "不动存储");
}

#[test]
fn remove_dry_run_prints_list_without_deleting_or_confirming() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  // confirm 注入为 panic——dry-run 优先级最高，不得触达确认
  let confirm = |_text: &str| panic!("dry-run 不得触达确认交互");
  for extra in [&["--dry-run"][..], &["--dry-run", "--yes"][..]] {
    let (out, err, code) = run_remove(
      &["token", "remove", "gitlab", "--all"]
        .iter()
        .chain(extra.iter())
        .copied()
        .collect::<Vec<_>>(),
      &store,
      Some(&confirm),
    );
    assert_eq!(code, 0, "{extra:?} 退出码：{err}");
    assert!(out.contains("dry run — no changes made"), "{out}");
    assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
    assert_eq!(
      read_token_store_at(&store).unwrap().len(),
      4,
      "{extra:?} 不产生写操作"
    );
  }
}

#[test]
fn remove_confirm_accept_deletes_after_listing() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let confirm = |text: &str| {
    assert_eq!(text, "Remove the listed tokens?", "确认文案");
    Ok(true)
  };
  let (out, _err, code) = run_remove(
    &["token", "remove", "gitlab", "--all"],
    &store,
    Some(&confirm),
  );
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("tokens to remove:"), "先列清单：{out}");
  assert!(out.contains("gitlab (https://gitlab-a.com)"), "{out}");
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens.len(), 1, "确认后删除：{tokens:?}");
  assert!(tokens.contains_key("gitee"));
}

#[test]
fn remove_confirm_decline_cancels_without_deleting() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let confirm = |_text: &str| Ok(false);
  let (out, _err, code) = run_remove(&["token", "remove", "gitlab"], &store, Some(&confirm));
  assert_eq!(code, 0, "拒绝非失败，退出码");
  assert!(out.contains("canceled"), "{out}");
  assert_eq!(read_token_store_at(&store).unwrap().len(), 4, "拒绝不删");
}

#[test]
fn remove_non_tty_without_yes_errors() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  // 真实 TTY 守卫实现的报错形态（非 TTY 无法交互确认）
  let confirm = |_text: &str| {
    Err(TokenError::Prompt {
      message:
        "cannot confirm interactively (not a TTY); re-run with --yes to delete without confirmation"
          .into(),
    })
  };
  let (_out, err, code) = run_remove(&["token", "remove", "gitlab"], &store, Some(&confirm));
  assert_eq!(code, 1, "非 TTY 无 --yes 报错，退出码");
  assert!(err.contains("--yes"), "报错引导 --yes：{err}");
  assert_eq!(read_token_store_at(&store).unwrap().len(), 4, "不能静默删");
}

#[test]
fn remove_without_confirm_injection_hits_real_tty_guard() {
  // 不注入 confirm——走真实 confirm_remove 的 TTY 守卫（测试环境非 TTY，
  // 守卫必须报错引导 --yes，不得静默删也不得阻塞）
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (_out, err, code) = run_remove(&["token", "remove", "gitlab"], &store, None);
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("not a TTY"), "{err}");
  assert!(err.contains("--yes"), "{err}");
  assert_eq!(read_token_store_at(&store).unwrap().len(), 4, "不能静默删");
}

#[test]
fn remove_host_and_all_conflict_errors() {
  let dir = TempDir::new().unwrap();
  let store = seeded_store(&dir);
  let (_out, err, code) = run_remove(
    &[
      "token",
      "remove",
      "gitlab",
      "--host",
      "https://gitlab-a.com",
      "--all",
      "--yes",
    ],
    &store,
    None,
  );
  assert_eq!(code, 1, "退出码");
  assert!(
    err.contains("options --host and --all cannot be used together"),
    "{err}"
  );
  assert_eq!(
    read_token_store_at(&store).unwrap().len(),
    4,
    "冲突不动存储"
  );
}

#[test]
fn remove_host_without_name_errors() {
  let dir = TempDir::new().unwrap();
  let (_out, err, code) = run(
    &["token", "remove", "--host", "https://gitlab-a.com"],
    &store_in(&dir),
  );
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("usage: vbumpp token remove <name>"), "{err}");
}

#[test]
fn remove_dry_run_missing_target_warns() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "x").unwrap();
  let (out, _err, code) = run_remove(&["token", "remove", "github", "--dry-run"], &store, None);
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("no token found for github"), "{out}");
}

#[test]
fn token_help_covers_remove_matrix_flags() {
  let dir = TempDir::new().unwrap();
  let (out, _err, code) = run(&["--help"], &store_in(&dir));
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("--host <url> for gitlab"), "{out}");
  assert!(out.contains("--all, --yes, --dry-run"), "{out}");
}

// ---------------------------------------------------------------------------
// flag 扫描 helper（scan_token_args 单元测试，镜像 src/cli/token.rs）
// ---------------------------------------------------------------------------

#[test]
fn scan_token_args_value_flag_both_forms() {
  let scanned = scan_token_args(&argv(&["gitlab", "--host", "a.com"]), &[], &["host"]).unwrap();
  assert_eq!(scanned.positionals, vec!["gitlab".to_string()]);
  assert_eq!(
    scanned.values.get("host").map(String::as_str),
    Some("a.com")
  );

  let scanned = scan_token_args(&argv(&["gitlab", "--host=a.com"]), &[], &["host"]).unwrap();
  assert_eq!(
    scanned.values.get("host").map(String::as_str),
    Some("a.com")
  );
}

#[test]
fn scan_token_args_double_dash_stops_flag_parsing() {
  let scanned = scan_token_args(&argv(&["--", "--host", "a.com"]), &[], &["host"]).unwrap();
  assert_eq!(
    scanned.positionals,
    vec!["--host".to_string(), "a.com".to_string()],
    "-- 之后一律位置参数"
  );
  assert!(scanned.values.is_empty());
}

#[test]
fn scan_token_args_unknown_flag_and_missing_value_error() {
  let err = scan_token_args(&argv(&["--wat"]), &[], &["host"]).unwrap_err();
  assert_eq!(err, "unknown option: --wat");
  let err = scan_token_args(&argv(&["-x"]), &[], &["host"]).unwrap_err();
  assert_eq!(err, "unknown option: -x");
  let err = scan_token_args(&argv(&["--host"]), &[], &["host"]).unwrap_err();
  assert_eq!(err, "option --host requires a value");
}

#[test]
fn scan_token_args_value_flag_rejects_flag_shaped_value() {
  // `--host --foo` 必为漏值笔误——flag 形态的下一参数不当值吞
  let err = scan_token_args(&argv(&["--host", "--foo"]), &[], &["host"]).unwrap_err();
  assert_eq!(err, "option --host requires a value");
  let err = scan_token_args(&argv(&["--host", "-x"]), &[], &["host"]).unwrap_err();
  assert_eq!(err, "option --host requires a value");
}

#[test]
fn scan_token_args_bool_flag_truthy_with_value() {
  // mri 惯例：布尔 flag 带值亦视为命中（与 bump/release 解析一致）
  let scanned = scan_token_args(&argv(&["--all=false"]), &["all"], &[]).unwrap();
  assert!(scanned.flags.contains("all"));
}
