//! versionBump 全链路编排——真实临时 git 仓库，对齐上游时序与事件序列。

mod common;

use std::fs;
use std::sync::{Arc, Mutex};

use tempfile::TempDir;
use vbumpp_core::bump::{version_bump, BumpOptions, CommitInput, Progress, Scripts, TagInput};
use vbumpp_core::progress::ProgressEvent;

fn init_repo(dir: &TempDir) -> std::path::PathBuf {
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  common::git(&path, &["config", "commit.gpgsign", "false"]);
  common::git(&path, &["config", "tag.gpgsign", "false"]);
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  path
}

fn base_options<'a>() -> BumpOptions<'a> {
  BumpOptions {
    release: Some("2.0.0"),
    files: vec![],
    recursive: false,
    commit: Some(CommitInput::Bool(true)),
    tag: Some(TagInput::Bool(true)),
    push: false,
    sign: false,
    all: false,
    no_verify: false,
    confirm: false,
    ignore_scripts: false,
    install: false,
    execute: None,
    scripts: None,
    preid: None,
    current_version: None,
  }
}

type EventLog = Arc<Mutex<Vec<(ProgressEvent, Option<String>)>>>;

fn collect_events() -> (EventLog, impl FnMut(&Progress)) {
  let events = Arc::new(Mutex::new(Vec::new()));
  let events2 = Arc::clone(&events);
  let cb = move |p: &Progress| {
    events2
      .lock()
      .unwrap()
      .push((p.event, p.script.map(str::to_owned)));
  };
  (events, cb)
}

#[test]
fn full_pipeline_files_commit_tag_and_events() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  // 两个配置声明的脚本槽位（ADR-0011）+ 一个文本文件
  fs::write(path.join("VERSION.txt"), "version 1.0.0\n").unwrap();
  common::git(&path, &["add", "."]); // 提交内文件须已跟踪（上游 pathspec 行为一致）
  common::git(&path, &["commit", "-m", "add files"]);
  let (events, mut cb) = collect_events();
  let pre_cmd = "node -e \"require('fs').writeFileSync('pre.txt','')\"";
  let post_cmd = "node -e \"require('fs').writeFileSync('post.txt','')\"";
  let opts = BumpOptions {
    files: vec!["package.json".to_string(), "VERSION.txt".to_string()],
    scripts: Some(Scripts {
      preversion: Some(pre_cmd.to_string()),
      postversion: Some(post_cmd.to_string()),
      ..Default::default()
    }),
    ..base_options()
  };
  let results = version_bump(&opts, &path, &mut cb).unwrap();

  // 文件已更新
  assert!(fs::read_to_string(path.join("package.json"))
    .unwrap()
    .contains("\"version\": \"2.0.0\""));
  assert_eq!(
    fs::read_to_string(path.join("VERSION.txt")).unwrap(),
    "version 2.0.0\n"
  );
  // commit 与 tag（上游默认信息模板）
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: release v2.0.0"
  );
  assert_eq!(common::git(&path, &["tag", "-l"]), "v2.0.0");
  // scripts 按序执行
  assert!(path.join("pre.txt").exists());
  assert!(path.join("post.txt").exists());
  // results 形状（上游 operation.results）
  assert_eq!(results.current_version, "1.0.0");
  assert_eq!(results.new_version, "2.0.0");
  assert_eq!(results.commit.as_deref(), Some("chore: release v2.0.0"));
  assert_eq!(results.tag.as_deref(), Some("v2.0.0"));
  assert_eq!(results.updated_files.len(), 2);
  assert!(results.skipped_files.is_empty());
  // 事件序列对齐上游时序
  let events = events.lock().unwrap();
  let kinds: Vec<ProgressEvent> = events.iter().map(|(e, _)| *e).collect();
  assert_eq!(
    kinds,
    vec![
      ProgressEvent::Script, // preversion
      ProgressEvent::FileUpdated,
      ProgressEvent::FileUpdated,
      ProgressEvent::GitCommit,
      ProgressEvent::GitTag,
      ProgressEvent::Script, // postversion
    ]
  );
  // 脚本事件负载为命令本体（ADR-0011）
  assert_eq!(events[0].1.as_deref(), Some(pre_cmd));
  assert_eq!(events[5].1.as_deref(), Some(post_cmd));
}

#[test]
fn push_pipeline_pushes_to_remote() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let bare = TempDir::new().unwrap();
  common::git(&path, &["init", "--bare", bare.path().to_str().unwrap()]);
  common::git(
    &path,
    &["remote", "add", "origin", bare.path().to_str().unwrap()],
  );
  common::git(&path, &["push", "-u", "origin", "main"]);
  let (events, mut cb) = collect_events();
  let opts = BumpOptions {
    files: vec!["package.json".to_string()],
    push: true,
    ..base_options()
  };
  version_bump(&opts, &path, &mut cb).unwrap();
  assert_eq!(
    common::git(bare.path(), &["log", "-1", "--pretty=%s", "main"]),
    "chore: release v2.0.0"
  );
  assert_eq!(common::git(bare.path(), &["tag", "-l"]), "v2.0.0");
  let kinds: Vec<ProgressEvent> = events.lock().unwrap().iter().map(|(e, _)| *e).collect();
  assert_eq!(kinds.last(), Some(&ProgressEvent::GitPush));
}

#[test]
fn empty_files_uses_default_manifest_list() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(
    path.join("package-lock.json"),
    "{\n  \"version\": \"1.0.0\",\n  \"packages\": {}\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "add lock"]);
  let (_events, mut cb) = collect_events();
  let opts = base_options(); // files 为空 → 默认清单
  version_bump(&opts, &path, &mut cb).unwrap();
  assert!(fs::read_to_string(path.join("package.json"))
    .unwrap()
    .contains("2.0.0"));
  assert!(fs::read_to_string(path.join("package-lock.json"))
    .unwrap()
    .contains("2.0.0"));
}

#[test]
fn glob_patterns_expand_and_sort() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::create_dir_all(path.join("packages/a")).unwrap();
  fs::write(
    path.join("packages/a/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "add sub package"]);
  let (_events, mut cb) = collect_events();
  let opts = BumpOptions {
    files: vec![
      "package.json".to_string(),
      "packages/**/package.json".to_string(),
    ],
    ..base_options()
  };
  version_bump(&opts, &path, &mut cb).unwrap();
  assert!(fs::read_to_string(path.join("packages/a/package.json"))
    .unwrap()
    .contains("2.0.0"));
}

#[test]
fn commit_false_skips_commit_but_tag_implies_commit() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let (events, mut cb) = collect_events();
  // 上游 normalizeOptions：tag 开启时 commit 对象强制存在（tag 需要承载提交）
  let opts = BumpOptions {
    commit: None,
    tag: Some(TagInput::Bool(true)),
    files: vec!["package.json".to_string()],
    ..base_options()
  };
  version_bump(&opts, &path, &mut cb).unwrap();
  assert_eq!(common::git(&path, &["tag", "-l"]), "v2.0.0");
  let kinds: Vec<ProgressEvent> = events.lock().unwrap().iter().map(|(e, _)| *e).collect();
  assert!(kinds.contains(&ProgressEvent::GitCommit), "tag 隐含 commit");
}

#[test]
fn ignore_scripts_skips_all_script_steps() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let (events, mut cb) = collect_events();
  let opts = BumpOptions {
    ignore_scripts: true,
    files: vec!["package.json".to_string()],
    scripts: Some(Scripts {
      preversion: Some("exit 1".to_string()),
      ..Default::default()
    }),
    ..base_options()
  };
  version_bump(&opts, &path, &mut cb).unwrap();
  assert!(events
    .lock()
    .unwrap()
    .iter()
    .all(|(e, _)| *e != ProgressEvent::Script));
}

#[test]
fn failing_script_aborts_bump() {
  // ADR-0011：配置声明的脚本非零退出即报错传播，发版中止
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let (_events, mut cb) = collect_events();
  let opts = BumpOptions {
    files: vec!["package.json".to_string()],
    scripts: Some(Scripts {
      preversion: Some("exit 1".to_string()),
      ..Default::default()
    }),
    ..base_options()
  };
  let err = version_bump(&opts, &path, &mut cb).unwrap_err();
  assert!(err.to_string().contains("exit 1"), "错误应含命令：{err}");
  // preversion 在 updateFiles 之前：文件未被改写
  assert!(fs::read_to_string(path.join("package.json"))
    .unwrap()
    .contains("1.0.0"));
}

#[test]
fn execute_runs_command_after_files() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let (_events, mut cb) = collect_events();
  let opts = BumpOptions {
    execute: Some("node -e \"require('fs').writeFileSync('executed.txt','')\""),
    files: vec!["package.json".to_string()],
    ..base_options()
  };
  version_bump(&opts, &path, &mut cb).unwrap();
  assert!(path.join("executed.txt").exists());
}

#[test]
fn failing_step_rejects_with_readable_error() {
  let dir = TempDir::new().unwrap();
  // 非 git 仓库：git commit 失败 → 错误可读且包含 git 输出
  fs::write(
    dir.path().join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  let (_events, mut cb) = collect_events();
  let opts = BumpOptions {
    files: vec!["package.json".to_string()],
    ..base_options()
  };
  let err = version_bump(&opts, dir.path(), &mut cb).unwrap_err();
  assert!(
    err.to_string().contains("not a git repository"),
    "错误应含 stderr：{err}"
  );
}

#[test]
fn progress_snapshot_matches_upstream_shape() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let snapshots = Arc::new(Mutex::new(Vec::new()));
  let snapshots2 = Arc::clone(&snapshots);
  let mut cb = move |p: &Progress| {
    snapshots2.lock().unwrap().push((
      p.event,
      p.new_version.to_owned(),
      p.updated_files.len(),
      p.skipped_files.len(),
      p.commit.map(str::to_owned),
      p.tag.map(str::to_owned),
    ));
  };
  let opts = BumpOptions {
    files: vec!["package.json".to_string()],
    ..base_options()
  };
  version_bump(&opts, &path, &mut cb).unwrap();
  let snaps = snapshots.lock().unwrap();
  // FileUpdated 事件时 updatedFiles 已含该文件（上游发送累计数组，消费端 pop 最后一个）
  let file_event = snaps
    .iter()
    .find(|(e, ..)| *e == ProgressEvent::FileUpdated)
    .unwrap();
  assert_eq!(
    (file_event.1.as_str(), file_event.2, file_event.3),
    ("2.0.0", 1, 0)
  );
  // GitCommit 事件负载含 commitMessage，tag 字段在 GitTag 前为 None（上游 false）
  let commit_event = snaps
    .iter()
    .find(|(e, ..)| *e == ProgressEvent::GitCommit)
    .unwrap();
  assert_eq!(commit_event.4.as_deref(), Some("chore: release v2.0.0"));
  // 上游：tag 启用时 GitTag 前的事件负载为 ""（state.tagName 初值）
  assert_eq!(commit_event.5.as_deref(), Some(""));
  let tag_event = snaps
    .iter()
    .find(|(e, ..)| *e == ProgressEvent::GitTag)
    .unwrap();
  assert_eq!(tag_event.5.as_deref(), Some("v2.0.0"));
}

#[test]
fn recursive_default_files_expand_packages_manifests() {
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  // 根 package.json 提供版本来源
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  fs::create_dir_all(path.join("packages/sub")).unwrap();
  fs::write(
    path.join("packages/sub/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);

  let mut options = base_options();
  options.recursive = true;
  options.commit = None;
  options.tag = None;
  let (_events, mut cb) = collect_events();
  let results = version_bump(&options, &path, &mut cb).unwrap();

  // ADR-0009：recursive 默认清单 = 链上 basename 表的 `**/` 整树收集模式，
  // 整树命中 packages 下的 manifest（替代上游 packages/**/package.json 硬编码）
  assert_eq!(results.new_version, "2.0.0");
  assert_eq!(results.updated_files.len(), 2);
  assert!(
    results.updated_files[1].ends_with("packages/sub/package.json"),
    "应递归命中 packages 下的 manifest：{}",
    results.updated_files[1]
  );
  assert_eq!(
    fs::read_to_string(path.join("packages/sub/package.json")).unwrap(),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
}

// ---------------------------------------------------------------------------
// COL-61：gitignore 感知收集 + commit 未跟踪 pathspec 兜底
// ---------------------------------------------------------------------------

#[test]
fn recursive_skips_gitignored_residue() {
  // COL-61 事故场景（v6.0.0 发版中断实例）：-r 整树收集须跳过 gitignored
  // 构建残留（target/package 打包暂存、.next 缓存），残留不被撞版本号、
  // 不进更新清单，commit 不再撞未跟踪 pathspec
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  fs::write(path.join(".gitignore"), "target/\n.next/\n").unwrap();
  fs::create_dir_all(path.join("target/package/demo-1.0.0")).unwrap();
  fs::write(
    path.join("target/package/demo-1.0.0/Cargo.toml"),
    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  )
  .unwrap();
  fs::create_dir_all(path.join("website/.next")).unwrap();
  fs::write(
    path.join("website/.next/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);

  let mut options = base_options();
  options.recursive = true;
  let (_events, mut cb) = collect_events();
  let results = version_bump(&options, &path, &mut cb).unwrap();

  assert_eq!(results.updated_files.len(), 1, "仅根 package.json 入列");
  assert_eq!(
    fs::read_to_string(path.join("package.json")).unwrap(),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
  for residue in [
    "target/package/demo-1.0.0/Cargo.toml",
    "website/.next/package.json",
  ] {
    assert!(
      fs::read_to_string(path.join(residue))
        .unwrap()
        .contains("1.0.0"),
      "gitignored 残留不应被撞版本号：{residue}"
    );
  }
  let committed = common::git(&path, &["show", "--pretty=format:", "--name-only", "HEAD"]);
  assert_eq!(committed, "package.json", "release commit 仅含已跟踪文件");
}

#[test]
fn commit_filters_untracked_files_with_warning() {
  // COL-61 兜底层：显式 files 引入的未跟踪文件不再炸 commit——滤出提交、
  // 磁盘修改保留（不静默丢弃的另一半是 ⚠ 警告，走 stdout 不在本 seam 断言）
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  // 初始提交之后才落盘——未跟踪（非 gitignored，兜底层与收集层解耦）
  fs::create_dir_all(path.join("nested")).unwrap();
  fs::write(
    path.join("nested/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();

  let options = BumpOptions {
    files: vec![
      "package.json".to_string(),
      "nested/package.json".to_string(),
    ],
    ..base_options()
  };
  let (_events, mut cb) = collect_events();
  let results = version_bump(&options, &path, &mut cb).unwrap();

  assert_eq!(results.updated_files.len(), 2, "两份文件都已更新");
  assert_eq!(
    fs::read_to_string(path.join("nested/package.json")).unwrap(),
    "{\n  \"version\": \"2.0.0\"\n}\n",
    "未跟踪文件保留磁盘修改"
  );
  let committed = common::git(&path, &["show", "--pretty=format:", "--name-only", "HEAD"]);
  assert_eq!(committed, "package.json", "commit 仅含已跟踪文件");
}

#[test]
fn explicit_files_bypass_gitignore_filter() {
  // COL-61 spec 边界：gitignore 过滤只作用于 glob 收集——字面点名的文件
  // 即使 gitignored 也照更（用户意图优先）；提交侧由兜底层过滤 + 警告
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  fs::write(path.join(".gitignore"), "generated/\n").unwrap();
  fs::create_dir_all(path.join("generated")).unwrap();
  fs::write(
    path.join("generated/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);

  let options = BumpOptions {
    files: vec![
      "package.json".to_string(),
      "generated/package.json".to_string(),
    ],
    ..base_options()
  };
  let (_events, mut cb) = collect_events();
  let results = version_bump(&options, &path, &mut cb).unwrap();

  assert_eq!(results.updated_files.len(), 2, "字面点名文件照更不误");
  assert_eq!(
    fs::read_to_string(path.join("generated/package.json")).unwrap(),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
  let committed = common::git(&path, &["show", "--pretty=format:", "--name-only", "HEAD"]);
  assert_eq!(committed, "package.json", "commit 仅含已跟踪文件（兜底层）");
}

#[test]
fn commit_tolerates_gitignored_cargo_lock() {
  // COL-61 兜底层同类第二实例：gitignored Cargo.lock 经 ADR-0003 定向同步
  // 入 updated_files——库 crate 常见布局，pre-fix 同样炸 pathspec 提交
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  fs::write(
    path.join("Cargo.toml"),
    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  )
  .unwrap();
  fs::write(path.join(".gitignore"), "Cargo.lock\n").unwrap();
  fs::write(
    path.join("Cargo.lock"),
    "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  )
  .unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);

  let options = BumpOptions {
    files: vec!["Cargo.toml".to_string()],
    ..base_options()
  };
  let (_events, mut cb) = collect_events();
  let results = version_bump(&options, &path, &mut cb).unwrap();

  assert_eq!(
    results.updated_files.len(),
    2,
    "Cargo.toml + Cargo.lock 同步"
  );
  assert!(
    fs::read_to_string(path.join("Cargo.lock"))
      .unwrap()
      .contains("2.0.0"),
    "gitignored Cargo.lock 保留磁盘修改"
  );
  let committed = common::git(&path, &["show", "--pretty=format:", "--name-only", "HEAD"]);
  assert_eq!(committed, "Cargo.toml", "commit 仅含已跟踪文件");
}

#[test]
fn non_git_dir_recursive_fails_open() {
  // COL-61 fail-open：非 git 目录下 gitignore 探测失败回落裸 walk（上游
  // parity 现状）——收集不过滤、不新增报错
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  fs::write(path.join(".gitignore"), "target/\n").unwrap();
  fs::create_dir_all(path.join("target/residue")).unwrap();
  fs::write(
    path.join("target/residue/package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();

  let mut options = base_options();
  options.recursive = true;
  options.commit = None;
  options.tag = None;
  let (_events, mut cb) = collect_events();
  let results = version_bump(&options, &path, &mut cb).unwrap();

  assert_eq!(results.updated_files.len(), 2, "非 git 目录回落不过滤");
  assert_eq!(
    fs::read_to_string(path.join("target/residue/package.json")).unwrap(),
    "{\n  \"version\": \"2.0.0\"\n}\n"
  );
}

// ---------------------------------------------------------------------------
// BumpOptions::from_merged（ADR-0014）：合并配置 → versionBump 输入的转换
// ---------------------------------------------------------------------------

#[test]
fn from_merged_maps_config_keys() {
  let merged = serde_json::json!({
    "files": ["package.json", "Cargo.toml"],
    "commit": "chore: custom v",
    "tag": "release-v",
    "push": true,
    "sign": true,
    "all": true,
    "noVerify": true,
    "confirm": false,
    "ignoreScripts": true,
    "install": true,
    "execute": "echo hi",
    "scripts": { "preversion": "echo pre", "version": "echo v", "postversion": "echo post" },
    "preid": "alpha",
    "currentVersion": "1.0.0",
  });
  let merged = merged.as_object().unwrap();
  let options = BumpOptions::from_merged(merged, "2.0.0");
  assert_eq!(options.release, Some("2.0.0"));
  assert_eq!(options.files, vec!["package.json", "Cargo.toml"]);
  assert!(matches!(
    options.commit,
    Some(CommitInput::Message("chore: custom v"))
  ));
  assert!(matches!(options.tag, Some(TagInput::Name("release-v"))));
  assert!(options.push && options.sign && options.all && options.no_verify);
  assert!(!options.confirm);
  assert!(options.ignore_scripts && options.install);
  assert_eq!(options.execute, Some("echo hi"));
  let scripts = options.scripts.unwrap();
  assert_eq!(scripts.preversion.as_deref(), Some("echo pre"));
  assert_eq!(scripts.version.as_deref(), Some("echo v"));
  assert_eq!(scripts.postversion.as_deref(), Some("echo post"));
  assert_eq!(options.preid, Some("alpha"));
  assert_eq!(options.current_version, Some("1.0.0"));
}

#[test]
fn from_merged_tolerates_missing_and_falsy() {
  // 空表 → 全默认；commit/tag 空字符串按上游 falsy 语义关闭
  let empty = serde_json::Map::new();
  let options = BumpOptions::from_merged(&empty, "1.0.1");
  assert_eq!(options.release, Some("1.0.1"));
  assert!(options.files.is_empty());
  assert!(options.commit.is_none());
  assert!(options.tag.is_none());
  assert!(!options.push && !options.sign && !options.install);

  let merged = serde_json::json!({ "commit": "", "tag": "", "files": "not-array" });
  let merged = merged.as_object().unwrap();
  let options = BumpOptions::from_merged(merged, "1.0.1");
  assert!(options.commit.is_none(), "空字符串 commit 关闭");
  assert!(options.tag.is_none(), "空字符串 tag 关闭");
  assert!(options.files.is_empty(), "类型不符按缺失处理");
}
