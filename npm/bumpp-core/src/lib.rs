#![deny(clippy::all)]

use std::path::PathBuf;

use napi_derive::napi;
use serde_json::{Map, Value};

/// Scaffold smoke-test export: proves the Rust → napi → JS link works end to end.
/// Real bumpp APIs land in later tickets (COL-8+).
#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

fn resolve_cwd(cwd: Option<String>) -> napi::Result<PathBuf> {
  match cwd {
    Some(c) => Ok(PathBuf::from(c)),
    None => Ok(std::env::current_dir()?),
  }
}

fn to_napi_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::from_reason(e.to_string())
}

/// 加载并合并 bumpp 配置（仅 JSON 配置文件），语义对齐上游 bumpp v11 的 `loadBumpConfig`。
#[napi]
pub fn load_bump_config(
  overrides: Option<Map<String, Value>>,
  cwd: Option<String>,
) -> napi::Result<Map<String, Value>> {
  bumpp_core::config::load_bump_config(overrides, &resolve_cwd(cwd)?).map_err(to_napi_err)
}

/// 文件版本更新结果（对齐上游 operation.state 的 updatedFiles / skippedFiles）
#[napi(object)]
pub struct UpdateFilesOutcome {
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  #[napi(js_name = "skippedFiles")]
  pub skipped_files: Vec<String>,
}

/// 更新文件中的版本号（manifest 保格式更新 + 文本模板替换），对齐上游 `updateFiles`。
#[napi]
pub fn update_files(
  files: Vec<String>,
  cwd: Option<String>,
  current_version: String,
  new_version: String,
) -> napi::Result<UpdateFilesOutcome> {
  let cwd = resolve_cwd(cwd)?;
  bumpp_core::files::update_files(&files, &cwd, &current_version, &new_version)
    .map(|o| UpdateFilesOutcome {
      updated_files: o.updated_files().iter().map(|s| s.to_string()).collect(),
      skipped_files: o.skipped_files().iter().map(|s| s.to_string()).collect(),
    })
    .map_err(to_napi_err)
}

#[napi(object)]
pub struct CommitSpec {
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  pub all: bool,
  #[napi(js_name = "noVerify")]
  pub no_verify: bool,
  pub sign: bool,
  pub message: String,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
}

#[napi(object)]
pub struct TagSpec {
  pub name: String,
  pub message: String,
  pub sign: bool,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
}

#[napi(object)]
pub struct GitCommitOutcome {
  pub event: ProgressEvent,
  #[napi(js_name = "commitMessage")]
  pub commit_message: String,
}

#[napi(object)]
pub struct GitTagOutcome {
  pub event: ProgressEvent,
  #[napi(js_name = "tagName")]
  pub tag_name: String,
}

#[napi(object)]
pub struct GitPushOutcome {
  pub event: ProgressEvent,
}

#[napi(object)]
pub struct NpmScriptOutcome {
  pub event: ProgressEvent,
  pub script: String,
}

/// git commit（shell out 到 git 二进制），对齐上游 `gitCommit`。
#[napi]
pub fn git_commit(cwd: Option<String>, spec: CommitSpec) -> napi::Result<GitCommitOutcome> {
  let core_spec = bumpp_core::git::CommitSpec {
    updated_files: &spec.updated_files,
    all: spec.all,
    no_verify: spec.no_verify,
    sign: spec.sign,
    message: &spec.message,
    new_version: &spec.new_version,
  };
  bumpp_core::git::git_commit(&resolve_cwd(cwd)?, &core_spec)
    .map(|(e, m)| GitCommitOutcome {
      event: e.into(),
      commit_message: m,
    })
    .map_err(to_napi_err)
}

/// git tag（附注），对齐上游 `gitTag`。
#[napi]
pub fn git_tag(cwd: Option<String>, spec: TagSpec) -> napi::Result<GitTagOutcome> {
  let core_spec = bumpp_core::git::TagSpec {
    name: &spec.name,
    message: &spec.message,
    sign: spec.sign,
    new_version: &spec.new_version,
  };
  bumpp_core::git::git_tag(&resolve_cwd(cwd)?, &core_spec)
    .map(|(e, n)| GitTagOutcome {
      event: e.into(),
      tag_name: n,
    })
    .map_err(to_napi_err)
}

/// git push（withTags 时追加 `git push --tags`），对齐上游 `gitPush`。
#[napi]
pub fn git_push(cwd: Option<String>, with_tags: bool) -> napi::Result<GitPushOutcome> {
  bumpp_core::git::git_push(&resolve_cwd(cwd)?, with_tags)
    .map(|e| GitPushOutcome {
      event: e.into(),
    })
    .map_err(to_napi_err)
}

/// 执行 package.json 中的 npm script（ignoreScripts 时不执行），对齐上游 `runNpmScript`。
/// 返回 null 表示未执行；脚本非零退出不传播（上游 parity）。
#[napi]
pub fn run_npm_script(
  cwd: Option<String>,
  script: String,
  ignore_scripts: bool,
) -> napi::Result<Option<NpmScriptOutcome>> {
  bumpp_core::scripts::run_npm_script(&resolve_cwd(cwd)?, &script, ignore_scripts)
    .map(|r| {
      r.map(|(e, s)| NpmScriptOutcome {
        event: e.into(),
        script: s,
      })
    })
    .map_err(to_napi_err)
}

#[napi(object)]
#[derive(Default)]
pub struct BumpInfoArg {
  pub release: Option<String>,
  pub files: Option<Vec<String>>,
  pub cwd: Option<String>,
  pub preid: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: Option<String>,
}

/// 上游 operation.state 的形状
#[napi(object)]
pub struct BumpState {
  pub release: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: String,
  #[napi(js_name = "currentVersionSource")]
  pub current_version_source: String,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
  #[napi(js_name = "commitMessage")]
  pub commit_message: String,
  #[napi(js_name = "tagName")]
  pub tag_name: String,
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  #[napi(js_name = "skippedFiles")]
  pub skipped_files: Vec<String>,
}

impl From<bumpp_core::info::BumpState> for BumpState {
  fn from(s: bumpp_core::info::BumpState) -> Self {
    Self {
      release: s.release,
      current_version: s.current_version,
      current_version_source: s.current_version_source,
      new_version: s.new_version,
      commit_message: s.commit_message,
      tag_name: s.tag_name,
      updated_files: s.updated_files,
      skipped_files: s.skipped_files,
    }
  }
}

/// 上游 versionBumpInfo 的返回形状：{ state }
#[napi(object)]
pub struct VersionBumpInfo {
  pub state: BumpState,
}

pub struct VersionBumpInfoTask {
  arg: Option<napi::Either<String, BumpInfoArg>>,
}

#[napi]
impl napi::Task for VersionBumpInfoTask {
  type Output = VersionBumpInfo;
  type JsValue = VersionBumpInfo;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let arg = match self.arg.take() {
      // 上游：字符串入参等价于 { release: arg }
      Some(napi::Either::A(release)) => BumpInfoArg {
        release: Some(release),
        ..Default::default()
      },
      Some(napi::Either::B(a)) => a,
      None => BumpInfoArg::default(),
    };
    let cwd = resolve_cwd(arg.cwd)?;
    let files = arg.files.unwrap_or_default();
    let options = bumpp_core::info::BumpInfoOptions {
      release: arg.release.as_deref(),
      files: &files,
      current_version: arg.current_version.as_deref(),
      preid: arg.preid.as_deref(),
    };
    bumpp_core::info::version_bump_info(&options, &cwd)
      .map(|s| VersionBumpInfo { state: s.into() })
      .map_err(to_napi_err)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

/// 计算 bump 信息（当前版本 + 新版本），必要时在 Rust 侧渲染交互 prompt。
/// 对齐上游 bumpp v11 的 `versionBumpInfo`。
#[napi]
pub fn version_bump_info(
  arg: Option<napi::Either<String, BumpInfoArg>>,
) -> napi::bindgen_prelude::AsyncTask<VersionBumpInfoTask> {
  napi::bindgen_prelude::AsyncTask::new(VersionBumpInfoTask { arg })
}

#[napi(object)]
#[derive(Default)]
pub struct VersionBumpOptions {
  pub release: Option<String>,
  pub files: Option<Vec<String>>,
  pub cwd: Option<String>,
  pub commit: Option<napi::Either<bool, String>>,
  pub tag: Option<napi::Either<bool, String>>,
  pub push: Option<bool>,
  pub sign: Option<bool>,
  pub all: Option<bool>,
  #[napi(js_name = "noVerify")]
  pub no_verify: Option<bool>,
  pub confirm: Option<bool>,
  #[napi(js_name = "ignoreScripts")]
  pub ignore_scripts: Option<bool>,
  pub install: Option<bool>,
  pub execute: Option<String>,
  pub preid: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: Option<String>,
  pub recursive: Option<bool>,
  pub progress: Option<napi::bindgen_prelude::Function<'static, VersionBumpProgress, ()>>,
}

/// 上游 `VersionBumpProgress` 负载形状
#[napi(object)]
pub struct VersionBumpProgress {
  pub event: ProgressEvent,
  pub script: Option<String>,
  pub release: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: String,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
  /// 上游 commit: string | false
  pub commit: napi::Either<String, bool>,
  /// 上游 tag: string | false
  pub tag: napi::Either<String, bool>,
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  #[napi(js_name = "skippedFiles")]
  pub skipped_files: Vec<String>,
}

/// 上游 `operation.results`
#[napi(object)]
pub struct VersionBumpResults {
  pub release: Option<String>,
  #[napi(js_name = "currentVersion")]
  pub current_version: String,
  #[napi(js_name = "newVersion")]
  pub new_version: String,
  pub commit: napi::Either<String, bool>,
  pub tag: napi::Either<String, bool>,
  #[napi(js_name = "updatedFiles")]
  pub updated_files: Vec<String>,
  #[napi(js_name = "skippedFiles")]
  pub skipped_files: Vec<String>,
}

// CalleeHandled=false：JS 回调单参（上游 progress(payload) 签名）
type ProgressTsfn = napi::threadsafe_function::ThreadsafeFunction<
  VersionBumpProgress,
  (),
  VersionBumpProgress,
  napi::Status,
  false,
>;

/// versionBump 的纯数据输入（Function 抽取后，跨线程安全）
pub struct BumpTaskData {
  release: Option<String>,
  files: Vec<String>,
  cwd: Option<String>,
  commit: Option<napi::Either<bool, String>>,
  tag: Option<napi::Either<bool, String>>,
  push: bool,
  sign: bool,
  all: bool,
  no_verify: bool,
  confirm: bool,
  ignore_scripts: bool,
  install: bool,
  execute: Option<String>,
  preid: Option<String>,
  current_version: Option<String>,
  recursive: bool,
}

pub struct VersionBumpTask {
  data: BumpTaskData,
  progress: Option<ProgressTsfn>,
}

impl From<VersionBumpOptions> for BumpTaskData {
  fn from(o: VersionBumpOptions) -> Self {
    Self {
      release: o.release,
      files: o.files.unwrap_or_default(),
      cwd: o.cwd,
      commit: o.commit,
      tag: o.tag,
      push: o.push.unwrap_or(false),
      sign: o.sign.unwrap_or(false),
      all: o.all.unwrap_or(false),
      no_verify: o.no_verify.unwrap_or(false),
      confirm: o.confirm.unwrap_or(false),
      ignore_scripts: o.ignore_scripts.unwrap_or(false),
      install: o.install.unwrap_or(false),
      execute: o.execute,
      preid: o.preid,
      current_version: o.current_version,
      recursive: o.recursive.unwrap_or(false),
    }
  }
}

fn to_napi_progress(p: &bumpp_core::bump::Progress) -> VersionBumpProgress {
  VersionBumpProgress {
    event: p.event.into(),
    script: p.script.map(str::to_owned),
    release: p.release.map(str::to_owned),
    current_version: p.current_version.to_string(),
    new_version: p.new_version.to_string(),
    commit: match p.commit {
      Some(m) => napi::Either::A(m.to_string()),
      None => napi::Either::B(false),
    },
    tag: match p.tag {
      Some(t) => napi::Either::A(t.to_string()),
      None => napi::Either::B(false),
    },
    updated_files: p.updated_files.to_vec(),
    skipped_files: p.skipped_files.to_vec(),
  }
}

#[napi]
impl napi::Task for VersionBumpTask {
  type Output = VersionBumpResults;
  type JsValue = VersionBumpResults;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let options = &self.data;
    let cwd = resolve_cwd(options.cwd.clone())?;
    let core_options = bumpp_core::bump::BumpOptions {
      release: options.release.as_deref(),
      files: options.files.clone(),
      recursive: options.recursive,
      commit: options.commit.as_ref().map(|c| match c {
        napi::Either::A(b) => bumpp_core::bump::CommitInput::Bool(*b),
        napi::Either::B(s) => bumpp_core::bump::CommitInput::Message(s.as_str()),
      }),
      tag: options.tag.as_ref().map(|t| match t {
        napi::Either::A(b) => bumpp_core::bump::TagInput::Bool(*b),
        napi::Either::B(s) => bumpp_core::bump::TagInput::Name(s.as_str()),
      }),
      push: options.push,
      sign: options.sign,
      all: options.all,
      no_verify: options.no_verify,
      confirm: options.confirm,
      ignore_scripts: options.ignore_scripts,
      install: options.install,
      execute: options.execute.as_deref(),
      preid: options.preid.as_deref(),
      current_version: options.current_version.as_deref(),
    };
    let tsfn = &self.progress;
    let mut emit = |p: &bumpp_core::bump::Progress| {
      if let Some(tsfn) = tsfn {
        // 会合通道：阻塞至 JS 回调在主线程执行完毕，
        // 保证事件严格按序送达（Promise resolve 之前尾部事件不丢失）
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        tsfn.call_with_return_value(
          to_napi_progress(p),
          napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
          move |_, _| {
            let _ = tx.send(());
            Ok(())
          },
        );
        let _ = rx.recv();
      }
    };
    let results =
      bumpp_core::bump::version_bump(&core_options, &cwd, &mut emit).map_err(to_napi_err)?;
    Ok(VersionBumpResults {
      release: results.release,
      current_version: results.current_version,
      new_version: results.new_version,
      commit: match results.commit {
        Some(m) => napi::Either::A(m),
        None => napi::Either::B(false),
      },
      tag: match results.tag {
        Some(t) => napi::Either::A(t),
        None => napi::Either::B(false),
      },
      updated_files: results.updated_files,
      skipped_files: results.skipped_files,
    })
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

/// 完整 bump 流程：文件更新 + npm scripts + git commit/tag/push，
/// 进度事件经 ThreadsafeFunction 实时回传（不阻塞事件循环）。对齐上游 `versionBump`。
#[napi]
pub fn version_bump(
  mut options: VersionBumpOptions,
) -> napi::Result<napi::bindgen_prelude::AsyncTask<VersionBumpTask>> {
  let progress = match options.progress.take() {
    Some(f) => Some(
      f.build_threadsafe_function::<VersionBumpProgress>()
        .callee_handled::<false>()
        .build_callback(|ctx| Ok(ctx.value))?,
    ),
    None => None,
  };
  // Function 留在本函数栈帧销毁；跨线程的只有纯数据与 TSFN
  Ok(napi::bindgen_prelude::AsyncTask::new(VersionBumpTask {
    data: options.into(),
    progress,
  }))
}

/// 上游 `ProgressEvent` 字符串枚举（npm/bump 消费侧的 switch 键）
#[napi(string_enum)]
#[derive(Clone, Copy)]
pub enum ProgressEvent {
  #[napi(value = "file updated")]
  FileUpdated,
  #[napi(value = "file skipped")]
  FileSkipped,
  #[napi(value = "git commit")]
  GitCommit,
  #[napi(value = "git tag")]
  GitTag,
  #[napi(value = "git push")]
  GitPush,
  #[napi(value = "npm script")]
  NpmScript,
}

impl From<bumpp_core::progress::ProgressEvent> for ProgressEvent {
  fn from(e: bumpp_core::progress::ProgressEvent) -> Self {
    use bumpp_core::progress::ProgressEvent as Core;
    match e {
      Core::FileUpdated => Self::FileUpdated,
      Core::FileSkipped => Self::FileSkipped,
      Core::GitCommit => Self::GitCommit,
      Core::GitTag => Self::GitTag,
      Core::GitPush => Self::GitPush,
      Core::NpmScript => Self::NpmScript,
    }
  }
}
