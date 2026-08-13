//! bump --dry-run（COL-85）：骑 COL-83 收口的同一流水线（预演与执行同路）——
//! 配置四层合并、glob 展开与 gitignore 过滤、版本读取与计算、changelog 生成
//! 全部照走；逐行打印执行计划（逐文件预演判定 / 版本与来源 / 将写盘清单 /
//! 脚本与命令文本 / git 动作完整文本 / changelog 全文预览 / --provider 时的
//! 平台 Release 预览），全程零写盘、零 git 写操作、零网络、零 success 行。
//! 交互语义：版本选择菜单保留（非 TTY 用例经 config release 键走非交互），
//! `Bump?` 确认跳过（confirm=true 的 fixture 在非 TTY 下 exit 0 即证明）。
//!
//! env 修改为进程全局：涉 token 用例经 ENV_LOCK 串行并净化（tests/release_dry_run.rs
//! 同先例）。位置说明同 release_dry_run：CLI 应用层通路测试，经 `run_at` 全链路。

mod common;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use tempfile::TempDir;
use vbumpp_core::cli::{run_at, RunEnv};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 入场串行 + 净化（token 环境变量与存储路径覆盖）
fn sanitized_env() -> MutexGuard<'static, ()> {
  let guard = ENV_LOCK.lock().unwrap();
  for key in [
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GITLAB_TOKEN",
    "GITEE_TOKEN",
    "GITCODE_TOKEN",
    "VBUMPP_TOKEN_STORE",
  ] {
    std::env::remove_var(key);
  }
  common::isolate_global_home();
  guard
}

/// 跑一轮 bump 默认命令，cwd 注入仓库目录，收集 stdout / stderr / 退出码
fn run_bump(argv: &[&str], cwd: &Path) -> (String, String, i32) {
  let argv: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
  let store = cwd.join("tokens.bin");
  let env = RunEnv {
    store: Some(&store),
    cwd: Some(cwd),
    prompt: None,
  };
  let mut out = Vec::new();
  let mut err = Vec::new();
  let code = run_at(&argv, None, &env, &mut out, &mut err);
  (
    String::from_utf8(out).unwrap(),
    String::from_utf8(err).unwrap(),
    code,
  )
}

#[test]
fn dry_run_prints_full_plan_without_side_effects() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  // confirm = true：dry-run 必须跳过 Bump?（非 TTY 下若弹确认必败，exit 0 即证明）
  let path = common::init_bump_repo(
    &dir,
    "release = \"minor\"\nconfirm = true\ninstall = true\nexecute = 'node -e \"doSomething()\"'\n\
     [scripts]\npreversion = \"touch marker.txt\"\nversion = \"echo version-slot\"\npostversion = \"echo post\"\n",
  );
  // install 检测锚点（提交进仓库，保持工作区干净基线）
  std::fs::write(path.join("pnpm-lock.yaml"), "").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: add lock"]);

  let (out, err, code) = run_bump(&["--dry-run"], &path);
  assert_eq!(code, 0, "校验通过 exit 0：{err}");
  assert!(err.is_empty(), "{err}");

  // 1. 开头标识 dry run；全程无 success 行、无 Bump? 确认
  assert!(out.contains("bump plan (dry run"), "{out}");
  assert!(!out.contains('✔'), "dry-run 不打印任何 success 行：{out}");
  assert!(!out.contains("Bump?"), "{out}");

  // 2. 逐文件预演判定（与真实执行同一代码段产出）
  assert!(out.contains("package.json: update → 1.1.0"), "{out}");

  // 3. 当前版本及其来源、新版本
  assert!(
    out.contains("current version: 1.0.0 (source: package.json)"),
    "{out}"
  );
  assert!(out.contains("new version: 1.1.0"), "{out}");

  // 4. 将写盘的文件清单（含 CHANGELOG.md）
  assert!(out.contains("files to write:"), "{out}");
  assert!(out.contains("package.json"), "{out}");
  assert!(out.contains("CHANGELOG.md"), "{out}");

  // 5. 将执行的脚本与命令逐条列出命令文本（均不执行）
  assert!(out.contains("preversion: touch marker.txt"), "{out}");
  assert!(out.contains("version: echo version-slot"), "{out}");
  assert!(out.contains("postversion: echo post"), "{out}");
  assert!(out.contains("install: pnpm install"), "{out}");
  assert!(out.contains("execute: node -e \"doSomething()\""), "{out}");

  // 6. git 动作完整文本（%s 替换后的 commit message / tag 名 / push 序列）
  assert!(out.contains("commit: chore: release v1.1.0"), "{out}");
  assert!(out.contains("tag: v1.1.0"), "{out}");
  assert!(out.contains("git push"), "{out}");
  assert!(out.contains("git push --tags"), "{out}");

  // 7. changelog 全文预览（生成但不落盘、不 commit）
  assert!(out.contains("## v1.1.0"), "{out}");

  // 副作用零发生：脚本 marker 未创建、install 未执行（无 node_modules）、
  // CHANGELOG.md 未落盘、git 零新提交零新 tag、工作区零脏文件
  assert!(!path.join("marker.txt").exists(), "脚本不得真实 spawn");
  assert!(!path.join("node_modules").exists(), "install 不得真实执行");
  assert!(!path.join("CHANGELOG.md").exists(), "changelog 不得落盘");
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: add lock",
    "git 零新提交"
  );
  assert_eq!(common::git(&path, &["tag", "-l"]), "v1.0.0", "git 零新 tag");
  assert_eq!(
    common::git(&path, &["status", "--porcelain"]),
    "",
    "工作区零脏文件"
  );
}

#[test]
fn verdict_lines_cover_all_three_states() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\n");
  // VERSION.txt 已是新版本 → up-to-date；ghost.txt 不存在 → missing；
  // package.json 1.0.0 → update。显式 files（argv 位置参数）
  std::fs::write(path.join("VERSION.txt"), "version 1.1.0\n").unwrap();

  let (out, err, code) = run_bump(
    &["VERSION.txt", "ghost.txt", "package.json", "--dry-run"],
    &path,
  );
  assert_eq!(code, 0, "{err}");
  assert!(out.contains("VERSION.txt: up-to-date"), "{out}");
  assert!(out.contains("ghost.txt: missing"), "{out}");
  assert!(out.contains("package.json: update → 1.1.0"), "{out}");
}

#[test]
fn no_tag_prints_changelog_skip_reason() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  // 同 init_bump_repo 形态但不打 tag（无历史 tag → changelog 生成跳过）
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  common::git(&path, &["config", "commit.gpgsign", "false"]);
  std::fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  std::fs::write(path.join(".vbumpprc.toml"), "release = \"minor\"\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);

  let (out, err, code) = run_bump(&["--dry-run"], &path);
  assert_eq!(code, 0, "{err}");
  // 无历史 tag：标注跳过原因（其余计划照常）
  assert!(out.contains("changelog: skipped"), "{out}");
  assert!(out.contains("tag"), "{out}");
  assert!(out.contains("package.json: update → 1.1.0"), "{out}");
}

#[test]
fn invalid_release_version_fails_exit_1() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"bogus\"\n");

  let (_out, err, code) = run_bump(&["--dry-run"], &path);
  assert_eq!(code, 1, "前置校验失败照常 exit 1");
  assert!(err.contains("invalid version: bogus"), "{err}");
}

#[test]
fn provider_combo_appends_release_preview_and_token_source() {
  let _guard = sanitized_env();
  std::env::set_var("GITEE_TOKEN", "bump-dry-token");
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\n");

  let (out, err, code) = run_bump(&["--dry-run", "--provider", "gitee"], &path);
  std::env::remove_var("GITEE_TOKEN");

  assert_eq!(code, 0, "{err}");
  // bump 计划在前、平台 Release 预览（COL-84 渲染）在后；token 报告来源
  assert!(out.contains("bump plan (dry run"), "{out}");
  assert!(out.contains("release plan (dry run"), "{out}");
  assert!(
    out.contains("token source: environment variable GITEE_TOKEN"),
    "{out}"
  );
  assert!(out.contains("provider: Gitee"), "{out}");
  assert!(out.contains("host: https://gitee.com/api/v5"), "{out}");
  assert!(out.contains("tag_name: v1.1.0"), "{out}");
  assert!(
    out.contains("POST https://gitee.com/api/v5/repos/owner/repo/releases"),
    "{out}"
  );
  // 明文 token 零泄漏
  assert!(!out.contains("bump-dry-token"), "{out}");
}

#[test]
fn provider_combo_missing_token_warns_exit_0() {
  let _guard = sanitized_env();
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\n");

  let (out, err, code) = run_bump(&["--dry-run", "--provider", "gitee"], &path);
  // COL-84 AC 在 bump 组合下同样成立：token 缺失降级为警告、预览照常 exit 0
  assert_eq!(code, 0, "token 缺失降级为警告，exit 0：{err}");
  assert!(err.is_empty(), "{err}");
  assert!(out.contains("bump plan (dry run"), "{out}");
  assert!(out.contains("release plan (dry run"), "{out}");
  assert!(
    out.contains("no Gitee token detected; run vbumpp token set gitee to add one"),
    "{out}"
  );
  assert!(
    out.contains("POST https://gitee.com/api/v5/repos/owner/repo/releases"),
    "{out}"
  );
}

#[test]
fn recursive_combo_collects_nested_manifests() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\n");
  std::fs::create_dir(path.join("sub")).unwrap();
  std::fs::write(
    path.join("sub/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();

  let (out, err, code) = run_bump(&["--dry-run", "-r"], &path);
  assert_eq!(code, 0, "{err}");
  // -r 整树收集：嵌套 manifest 命中并逐行预演判定
  assert!(out.contains("package.json: update → 1.1.0"), "{out}");
  assert!(out.contains("sub/package.json: update → 1.1.0"), "{out}");
}

#[test]
fn custom_output_combo() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\n");

  let (out, err, code) = run_bump(&["--dry-run", "-o", "HISTORY.md"], &path);
  assert_eq!(code, 0, "{err}");
  // -o 组合一致：写盘清单指向自定义 changelog 文件，全文预览照常且不真实落盘
  assert!(out.contains("HISTORY.md"), "{out}");
  assert!(out.contains("## v1.1.0"), "{out}");
  assert!(!path.join("HISTORY.md").exists());
}

#[test]
fn help_lists_dry_run_for_bump() {
  let dir = TempDir::new().unwrap();
  let (out, _err, code) = run_bump(&["--help"], dir.path());
  assert_eq!(code, 0, "退出码");
  assert!(out.contains("--dry-run"), "{out}");
}
