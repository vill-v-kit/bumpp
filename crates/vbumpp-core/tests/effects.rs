//! 效应边界（COL-83）：bump / changelog / release 全链的副作用统一经 `Effects`
//! 注入执行——记录型实现（spy）骑同一条流水线（预演与执行同路的结构验证）：
//! 判定、计算产物、事件序列与真实执行一致；副作用被边界拦截
//! （零写盘 / 零 spawn / 零 HTTP，含 gitlab 的 GET project id）。

mod common;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;
use tempfile::TempDir;
use vbumpp_core::bump::{version_bump_with, BumpOptions, CommitInput, Progress, Scripts, TagInput};
use vbumpp_core::changelog::{generate_changelog_with, GenerateChangelogOptions};
use vbumpp_core::effects::{Effects, HttpResponse};
use vbumpp_core::exec::ExecError;
use vbumpp_core::plugins::update_files_with;
use vbumpp_core::progress::ProgressEvent;
use vbumpp_core::release::{create_release_with, Provider};

/// 记录型效应实现：全部副作用只记录不执行；HTTP 返回合成响应供流水线续走
/// （gitlab 的 GET project id 应答 `{"id":42}`，验证 POST URL 的消费链）
#[derive(Default)]
struct Spy {
  runs: Mutex<Vec<(String, Vec<String>)>>,
  writes: Mutex<Vec<(PathBuf, String)>>,
  http_calls: Mutex<Vec<(String, String, Value)>>,
}

impl Spy {
  fn runs(&self) -> Vec<(String, Vec<String>)> {
    self.runs.lock().unwrap().clone()
  }
  fn writes(&self) -> Vec<(PathBuf, String)> {
    self.writes.lock().unwrap().clone()
  }
  fn http_calls(&self) -> Vec<(String, String, Value)> {
    self.http_calls.lock().unwrap().clone()
  }
}

impl Effects for Spy {
  fn write_file(&self, path: &Path, content: &str) -> io::Result<()> {
    self
      .writes
      .lock()
      .unwrap()
      .push((path.to_path_buf(), content.to_owned()));
    Ok(())
  }

  fn run(&self, program: &str, args: &[String], _cwd: &Path) -> Result<(), ExecError> {
    self
      .runs
      .lock()
      .unwrap()
      .push((program.to_owned(), args.to_vec()));
    Ok(())
  }

  fn http_get(&self, url: &str, _headers: &[(&str, String)]) -> Result<HttpResponse, String> {
    self
      .http_calls
      .lock()
      .unwrap()
      .push(("GET".to_owned(), url.to_owned(), Value::Null));
    Ok(HttpResponse {
      status: 200,
      body: r#"{"id":42}"#.to_owned(),
    })
  }

  fn http_post_json(
    &self,
    url: &str,
    _headers: &[(&str, String)],
    body: &Value,
  ) -> Result<HttpResponse, String> {
    self
      .http_calls
      .lock()
      .unwrap()
      .push(("POST".to_owned(), url.to_owned(), body.clone()));
    Ok(HttpResponse {
      status: 201,
      body: "{}".to_owned(),
    })
  }
}

fn git_repo(dir: &TempDir) -> PathBuf {
  let path = dir.path().to_path_buf();
  common::git(&path, &["init", "-b", "main"]);
  common::git(&path, &["config", "user.email", "test@example.com"]);
  common::git(&path, &["config", "user.name", "Test"]);
  common::git(&path, &["config", "commit.gpgsign", "false"]);
  common::git(&path, &["config", "tag.gpgsign", "false"]);
  path
}

#[test]
fn bump_pipeline_rides_effect_boundary() {
  let dir = TempDir::new().unwrap();
  let path = git_repo(&dir);
  std::fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  std::fs::write(path.join("VERSION.txt"), "version 1.0.0\n").unwrap();
  common::git(&path, &["add", "."]); // 提交内文件须已跟踪（pathspec 行为一致）
  common::git(&path, &["commit", "-m", "chore: init"]);

  let spy = Spy::default();
  let events = std::sync::Arc::new(Mutex::new(Vec::new()));
  let events2 = std::sync::Arc::clone(&events);
  let mut cb = move |p: &Progress| events2.lock().unwrap().push(p.event);
  let options = BumpOptions {
    release: Some("2.0.0"),
    files: vec!["package.json".to_string(), "VERSION.txt".to_string()],
    recursive: false,
    commit: Some(CommitInput::Bool(true)),
    tag: Some(TagInput::Bool(true)),
    push: true,
    sign: false,
    all: false,
    no_verify: false,
    confirm: false,
    ignore_scripts: false,
    install: false,
    execute: Some("node -e \"doSomething()\""),
    scripts: Some(Scripts {
      preversion: Some("echo pre".to_string()),
      postversion: Some("echo post".to_string()),
      ..Default::default()
    }),
    preid: None,
    current_version: None,
  };
  let results = version_bump_with(&spy, &options, &path, &mut cb).unwrap();

  // 计算产物与真实执行一致（commit message / tag 名 / 文件清单）
  assert_eq!(results.current_version, "1.0.0");
  assert_eq!(results.new_version, "2.0.0");
  assert_eq!(results.commit.as_deref(), Some("chore: release v2.0.0"));
  assert_eq!(results.tag.as_deref(), Some("v2.0.0"));
  assert_eq!(results.updated_files.len(), 2);

  // 事件序列与真实执行一致（上游时序）
  assert_eq!(
    *events.lock().unwrap(),
    vec![
      ProgressEvent::Script, // preversion
      ProgressEvent::FileUpdated,
      ProgressEvent::FileUpdated,
      ProgressEvent::GitCommit,
      ProgressEvent::GitTag,
      ProgressEvent::Script, // postversion
      ProgressEvent::GitPush,
    ]
  );

  // spawn 全序列：argv 与真实执行逐字节一致（scripts / execute / commit / tag / push）
  // （pathspec 顺序 = normalize_files 排序后的 updated_files：VERSION.txt 在前）
  let pkg = path.join("package.json").to_string_lossy().into_owned();
  let txt = path.join("VERSION.txt").to_string_lossy().into_owned();
  assert_eq!(
    spy.runs(),
    vec![
      (
        "sh".to_string(),
        vec!["-c".to_string(), "echo pre".to_string()]
      ),
      (
        "node".to_string(),
        vec!["-e".to_string(), "doSomething()".to_string()]
      ),
      (
        "git".to_string(),
        vec![
          "commit".to_string(),
          "--allow-empty".to_string(),
          "--message".to_string(),
          "chore: release v2.0.0".to_string(),
          txt,
          pkg,
        ]
      ),
      (
        "git".to_string(),
        vec![
          "tag".to_string(),
          "--annotate".to_string(),
          "--message".to_string(),
          "chore: release v2.0.0".to_string(),
          "v2.0.0".to_string(),
        ]
      ),
      (
        "sh".to_string(),
        vec!["-c".to_string(), "echo post".to_string()]
      ),
      ("git".to_string(), vec!["push".to_string()]),
      (
        "git".to_string(),
        vec!["push".to_string(), "--tags".to_string()]
      ),
    ]
  );

  // 写盘全序列：内容为判定段算出的保格式全文（顺序同上，按排序后清单）
  let writes = spy.writes();
  assert_eq!(writes.len(), 2);
  assert_eq!(writes[0].0, path.join("VERSION.txt"));
  assert_eq!(writes[0].1, "version 2.0.0\n");
  assert_eq!(writes[1].0, path.join("package.json"));
  assert!(writes[1].1.contains("\"version\": \"2.0.0\""));

  // 边界拦截：磁盘零改动，git 零新提交零新 tag（scripts / execute 未真实 spawn）
  assert!(
    std::fs::read_to_string(path.join("package.json"))
      .unwrap()
      .contains("1.0.0"),
    "package.json 不应被真实改写"
  );
  assert_eq!(
    std::fs::read_to_string(path.join("VERSION.txt")).unwrap(),
    "version 1.0.0\n"
  );
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "chore: init"
  );
  assert!(common::git(&path, &["tag", "-l"]).is_empty());
}

#[test]
fn update_files_planning_is_read_only_and_writes_flow_through_boundary() {
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  // Cargo.toml 带动 Cargo.lock 定向同步（附带写盘条目，ADR-0003）
  std::fs::write(
    path.join("Cargo.toml"),
    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  )
  .unwrap();
  std::fs::write(
    path.join("Cargo.lock"),
    "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n",
  )
  .unwrap();
  // 已是新版本的 manifest → up-to-date 跳过；ghost.txt → missing 跳过
  std::fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"2.0.0\"\n}\n",
  )
  .unwrap();

  let spy = Spy::default();
  let files = vec![
    "Cargo.toml".to_string(),
    "package.json".to_string(),
    "ghost.txt".to_string(),
  ];
  let outcome = update_files_with(&spy, &files, &path, "1.0.0", "2.0.0").unwrap();

  // 判定三态与真实执行一致（update / up-to-date / missing，按处理顺序）
  let events: Vec<_> = outcome
    .events()
    .iter()
    .map(|(e, p)| (*e, p.rsplit('/').next().unwrap().to_owned()))
    .collect();
  assert_eq!(
    events,
    vec![
      (ProgressEvent::FileUpdated, "Cargo.toml".to_string()),
      (ProgressEvent::FileUpdated, "Cargo.lock".to_string()),
      (ProgressEvent::FileSkipped, "package.json".to_string()),
      (ProgressEvent::FileSkipped, "ghost.txt".to_string()),
    ]
  );

  // 写盘条目 = 判定计划产物（主文件 + 附带 lock），内容为判定段算出的全文
  let writes = spy.writes();
  assert_eq!(writes.len(), 2);
  assert_eq!(writes[0].0, path.join("Cargo.toml"));
  assert!(writes[0].1.contains("version = \"2.0.0\""));
  assert_eq!(writes[1].0, path.join("Cargo.lock"));
  assert!(writes[1].1.contains("version = \"2.0.0\""));

  // 判定段只读：真实磁盘零改动（含附带 lock），全程零 spawn
  assert!(
    std::fs::read_to_string(path.join("Cargo.toml"))
      .unwrap()
      .contains("1.0.0"),
    "Cargo.toml 不应被真实改写"
  );
  assert!(
    std::fs::read_to_string(path.join("Cargo.lock"))
      .unwrap()
      .contains("1.0.0"),
    "Cargo.lock 不应被真实改写"
  );
  assert!(spy.runs().is_empty());
}

#[test]
fn changelog_generation_rides_effect_boundary() {
  // 与 tests/changelog_generate.rs 相同的 fixture（tag + 两个 conventional 提交）
  let dir = TempDir::new().unwrap();
  let path = git_repo(&dir);
  common::isolate_global_home();
  std::fs::write(path.join("f.txt"), "init\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  common::git(&path, &["tag", "v1.0.0"]);
  common::git(
    &path,
    &["remote", "add", "origin", "git@github.com:owner/repo.git"],
  );
  std::fs::write(path.join("a.txt"), "a\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "feat(ui): add x (#12)"]);
  std::fs::write(path.join("b.txt"), "b\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "fix: repair y"]);

  let spy = Spy::default();
  let options = GenerateChangelogOptions {
    overrides: None,
    from: "v1.0.0".to_string(),
    to: "1.1.0".to_string(),
  };
  let outcome = generate_changelog_with(&spy, &options, &path).unwrap();

  // 生成产物与真实执行一致（markdown 与最终全文由纯计算段产出）
  assert!(
    outcome.markdown.starts_with("## v1.1.0"),
    "{}",
    outcome.markdown
  );
  assert!(outcome.changelog_md.contains(&outcome.markdown));

  // 写盘与 git add/commit 与真实执行一致（commit message 模板 {{output}} 已替换）
  let writes = spy.writes();
  assert_eq!(writes.len(), 1);
  assert_eq!(writes[0].0, path.join("CHANGELOG.md"));
  assert_eq!(
    writes[0].1, outcome.changelog_md,
    "写盘内容即返回的最终全文"
  );
  assert_eq!(
    spy.runs(),
    vec![
      (
        "git".to_string(),
        vec!["add".to_string(), "CHANGELOG.md".to_string()]
      ),
      (
        "git".to_string(),
        vec![
          "commit".to_string(),
          "-m".to_string(),
          "chore: update CHANGELOG.md".to_string(),
        ]
      ),
    ]
  );

  // 边界拦截：CHANGELOG.md 未落盘，git 仓库零新提交
  assert!(!path.join("CHANGELOG.md").exists());
  assert_eq!(
    common::git(&path, &["log", "-1", "--pretty=%s"]),
    "fix: repair y"
  );
}

#[test]
fn release_http_flows_through_boundary() {
  // token 经环境变量注入（本测试二进制内仅此与 gitlab 用例消费 env，键不同无竞态）
  std::env::set_var("GITEE_TOKEN", "spy-token");
  let dir = TempDir::new().unwrap();
  let path = git_repo(&dir);
  common::isolate_global_home();
  std::fs::write(path.join("f.txt"), "x\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  common::git(
    &path,
    &["remote", "add", "origin", "git@gitee.com:owner/repo.git"],
  );

  let spy = Spy::default();
  create_release_with(&spy, Provider::Gitee, "2.0.0", "release notes", &path, None).unwrap();

  // 请求构造与真实执行一致（URL / 共享请求体 / token 注入形态）
  let calls = spy.http_calls();
  assert_eq!(calls.len(), 1);
  assert_eq!(calls[0].0, "POST");
  assert_eq!(
    calls[0].1,
    "https://gitee.com/api/v5/repos/owner/repo/releases"
  );
  assert_eq!(calls[0].2["name"], "2.0.0");
  assert_eq!(calls[0].2["tag_name"], "v2.0.0");
  assert_eq!(calls[0].2["body"], "release notes");
  assert_eq!(calls[0].2["access_token"], "spy-token");
  assert_eq!(calls[0].2["target_commitish"], "main");

  // 边界拦截：零真实 HTTP、零写盘零 spawn
  assert!(spy.runs().is_empty());
  assert!(spy.writes().is_empty());
  std::env::remove_var("GITEE_TOKEN");
}

#[test]
fn gitlab_project_id_lookup_flows_through_boundary() {
  std::env::set_var("GITLAB_TOKEN", "spy-gl-token");
  let dir = TempDir::new().unwrap();
  let path = git_repo(&dir);
  common::isolate_global_home();
  std::fs::write(path.join("f.txt"), "x\n").unwrap();
  common::git(&path, &["add", "."]);
  common::git(&path, &["commit", "-m", "chore: init"]);
  common::git(
    &path,
    &["remote", "add", "origin", "git@gitlab.com:owner/repo.git"],
  );

  let spy = Spy::default();
  create_release_with(&spy, Provider::Gitlab, "2.0.0", "notes", &path, None).unwrap();

  // GET project id 与 POST releases 两跳都被边界拦截；POST URL 消费 GET 的 id
  let calls = spy.http_calls();
  assert_eq!(calls.len(), 2);
  assert_eq!(calls[0].0, "GET");
  assert_eq!(
    calls[0].1,
    "https://gitlab.com/api/v4/projects/owner%2Frepo"
  );
  assert_eq!(calls[1].0, "POST");
  assert_eq!(calls[1].1, "https://gitlab.com/api/v4/projects/42/releases");
  assert_eq!(calls[1].2["tag_name"], "v2.0.0");
  assert_eq!(calls[1].2["description"], "notes");
  std::env::remove_var("GITLAB_TOKEN");
}
