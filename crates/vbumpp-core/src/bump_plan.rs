//! bump 计划预览（COL-85 dry-run 的核心）：记录型效应（Recorder）骑完整编排
//! （`bump_version_at`，预演与执行同路）——配置合并、glob 收集、版本读取与
//! 计算、changelog 生成、逐文件判定、git 动作格式化、平台 Release 请求构造
//! 全部与真实执行一致，全部副作用（写盘 / spawn / HTTP）在边界拦截为记录
//! 条目（零磁盘、零 git 写操作、零网络）。装配产物 `BumpPlan` 携全部计划
//! 行所需的只读字段；明文 token 不出本函数（拦截 URL 装配时经
//! `release::scrub_token` 同规则脱敏；gitlab host 反推脱敏前做——脱敏只动
//! token 子串，不动 URL 路径段）。

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

use crate::effects::{Effects, HttpResponse};
use crate::exec::ExecError;
use crate::orchestrate::{BumpVersionOptions, BumpVersionOutcome, OrchestrateError};
use crate::plugins::FileVerdict;

/// 拦截的一次 spawn（记录序列保序；装配按 execute → install → scripts →
/// 其余（git）四类淘汰式分类——分类规则见 `plan_bump`）
#[derive(Debug, Clone)]
struct RecordedRun {
  program: String,
  args: Vec<String>,
}

/// bump dry-run 的装配产物：token 来源（无明文）+ 计划行数据源
#[derive(Debug)]
pub struct BumpPlan {
  /// 逐文件预演判定（更新文件相对显示形态 + 三态）：与真实执行同一代码段产出
  pub verdicts: Vec<(String, FileVerdict)>,
  pub current_version: String,
  /// 来源（上游 `user` = config 显式 currentVersion；否则来源文件名）
  pub current_version_source: String,
  pub new_version: String,
  /// 将写盘的文件（排序去重，含 CHANGELOG.md 与 Cargo.lock 定向同步）
  pub writes: Vec<PathBuf>,
  /// execute 命令（原始文本，shell tokenize 前——配置形态即展示形态）
  pub execute: Option<String>,
  /// install 命令（`pnpm install` / `cargo check --workspace`——检测已在
  /// 链内完成，记录 argv 即真实 argv）
  pub installs: Vec<String>,
  /// scripts 三槽位命令（槽位 + 命令文本，槽位序 preversion → version →
  /// postversion；未知外壳记录不进入此列——不在配置声明槽位内）
  pub scripts: Vec<(String, String)>,
  /// `%s` 替换后的 commit message（commit 未启用为 None）
  pub commit_message: Option<String>,
  /// tag 名（tag 未启用为 None）
  pub tag_name: Option<String>,
  /// push 拦截条目（git push → git push --tags 序；push 未启用为空）
  pub pushes: Vec<String>,
  /// 编排产出的 changelog 版本节（无历史 tag 为 None——渲染层标注跳过原因）
  pub changelog: Option<String>,
  /// 平台 Release 计划（--provider 传入时）：COL-84 同一装配产物与渲染
  pub release: Option<crate::release::ReleasePlan>,
}

/// 记录型效应（dry-run 注入）：全部副作用只记录不执行；HTTP 应答合成响应
/// 供 gitlab 链续走（POST URL 消费该 id——展示的 id 为占位值，真实 id 只有
/// 真实执行才能查得）
#[derive(Default)]
struct Recorder {
  runs: Mutex<Vec<RecordedRun>>,
  writes: Mutex<Vec<PathBuf>>,
  http: Mutex<Vec<crate::release::PlannedRequest>>,
}

impl Recorder {
  fn runs(&self) -> Vec<RecordedRun> {
    self.runs.lock().unwrap().clone()
  }

  fn writes(&self) -> Vec<PathBuf> {
    self.writes.lock().unwrap().clone()
  }

  fn http(&self) -> Vec<crate::release::PlannedRequest> {
    self.http.lock().unwrap().clone()
  }

  /// 清空 HTTP 记录（bump 段应为空——release 宽容段 dispatch 前的防御性
  /// 归零，保证计划的请求序列只取本段拦截）
  fn clear_http(&self) {
    self.http.lock().unwrap().clear();
  }
}

impl Effects for Recorder {
  fn write_file(&self, path: &Path, _content: &str) -> io::Result<()> {
    self.writes.lock().unwrap().push(path.to_path_buf());
    Ok(())
  }

  fn run(&self, program: &str, args: &[String], _cwd: &Path) -> Result<(), ExecError> {
    self.runs.lock().unwrap().push(RecordedRun {
      program: program.to_owned(),
      args: args.to_vec(),
    });
    Ok(())
  }

  fn http_get(&self, url: &str, _headers: &[(&str, String)]) -> Result<HttpResponse, String> {
    self
      .http
      .lock()
      .unwrap()
      .push(crate::release::PlannedRequest {
        method: "GET",
        url: url.to_owned(),
      });
    Ok(HttpResponse {
      status: 200,
      body: r#"{"id":0}"#.to_owned(),
    })
  }

  fn http_post_json(
    &self,
    url: &str,
    _headers: &[(&str, String)],
    _body: &Value,
  ) -> Result<HttpResponse, String> {
    self
      .http
      .lock()
      .unwrap()
      .push(crate::release::PlannedRequest {
        method: "POST",
        url: url.to_owned(),
      });
    Ok(HttpResponse {
      status: 201,
      body: "{}".to_owned(),
    })
  }
}

/// bump dry-run 的计划装配：记录型效应骑完整编排。bump 主体与真实编排
/// 逐段一致（`bump_version_at` provider 置 None 跳过其严格 release 段——
/// 严格 token 解析缺失即 exit 1 会违反 dry-run 缺失警告 exit 0 语义，
/// COL-84 AC）；release 计划由宽容段补走（同一 dispatch 链 + 记录型效应）。
/// 从拦截记录淘汰式分类装配计划行
pub fn plan_bump(options: &BumpVersionOptions, cwd: &Path) -> Result<BumpPlan, OrchestrateError> {
  let recorder = Recorder::default();
  let bump_only = BumpVersionOptions {
    overrides: options.overrides.clone(),
    provider: None,
  };
  let outcome: BumpVersionOutcome = crate::orchestrate::bump_version_at(
    &recorder,
    &bump_only,
    cwd,
    // 显示汇捕获：进度行（✔/ℹ 前缀）不进入 BumpPlan——计划以独立的
    // 「无 success 行」形态重新渲染（判定行数据源是 verdicts，非进度行）
    &mut |_| {},
  )?;

  // release 宽容段（--provider 时）：token 宽容解析与 dispatch 收在 release
  // 模块内（plan_release_dispatch——明文 token 不出模块，ADR-0014）→ 装配
  // COL-84 同形计划
  let release = match options.provider {
    Some(provider) => {
      let markdown = outcome
        .changelog
        .as_ref()
        .map(|c| c.markdown.as_str())
        .unwrap_or("");
      recorder.clear_http();
      let resolved = crate::release::plan_release_dispatch(
        &recorder,
        provider,
        &outcome.state.new_version,
        markdown,
        cwd,
        options.overrides.as_ref(),
      )?;
      Some(crate::release::assemble_plan(
        provider,
        &outcome.state.new_version,
        markdown,
        cwd,
        resolved.as_ref(),
        recorder.http(),
      )?)
    }
    None => None,
  };

  // ---- 拦截命令分类依据（配置原文；与链内消费同一 merged 语义）----
  let merged = crate::config::load_bump_config(options.overrides.clone(), cwd)?;
  let execute_text = merged
    .get("execute")
    .and_then(Value::as_str)
    .filter(|s| !s.is_empty())
    .map(str::to_owned);
  // execute 的判定 token：链内 shell_words tokenize 的首个 token（空 execute
  // 已在链内报错，此处只做分类）
  let execute_program = execute_text
    .as_deref()
    .and_then(|e| shell_words::split(e).ok())
    .and_then(|parts| parts.first().cloned());
  let scripts_config: Vec<(String, String)> = merged
    .get("scripts")
    .and_then(Value::as_object)
    .map(|s| {
      [
        ("preversion", s.get("preversion")),
        ("version", s.get("version")),
        ("postversion", s.get("postversion")),
      ]
      .into_iter()
      .filter_map(|(slot, value)| {
        value
          .and_then(Value::as_str)
          .map(|c| (slot.to_string(), c.to_string()))
      })
      .collect()
    })
    .unwrap_or_default();

  // ---- 淘汰式分类（记录序列保序；同记录不重复归类）----
  let mut execute = None;
  let mut installs = Vec::new();
  let mut shell_commands = Vec::new();
  let mut git_runs = Vec::new();
  for run in recorder.runs() {
    let args: Vec<&str> = run.args.iter().map(String::as_str).collect();
    // script 外壳形态：`sh -c <cmd>`（Unix）/ `cmd /d /s /c <cmd>`（Windows）
    let is_script_shell = (run.program == "sh" && args.len() == 2 && args[0] == "-c")
      || (run.program == "cmd" && args.len() == 4 && args[..3] == ["/d", "/s", "/c"]);
    if is_script_shell {
      shell_commands.push(run.args.last().cloned().unwrap_or_default());
      continue;
    }
    // execute：链内 shell tokenize 的实际 argv（首个 token 命中；外壳形态
    // 已在上分流——execute = "sh …" 不会吃掉 scripts 的 sh -c 记录）
    if execute_program.as_deref().is_some_and(|p| p == run.program) && execute.is_none() {
      execute = Some(execute_text.clone().unwrap_or_default());
      continue;
    }
    // install 两形态（ADR-0007 适配命令全集）：`<pm> install` / `cargo check --workspace`
    if args == ["install"] || args == ["check", "--workspace"] {
      installs.push(format!("{} {}", run.program, args.join(" ")));
      continue;
    }
    git_runs.push(run);
  }
  // scripts 槽位序（preversion → version → postversion）：配置原文匹配摘除
  // 后未知外壳记录不进计划（不在声明槽位内——与真实执行的槽位触发一致）
  let mut scripts: Vec<(String, String)> = Vec::new();
  for (slot, command) in &scripts_config {
    if let Some(index) = shell_commands.iter().position(|c| c == command) {
      shell_commands.remove(index);
      scripts.push((slot.clone(), command.clone()));
    }
  }

  // push 序列（git push → git push --tags，记录序即真实序）
  let pushes: Vec<String> = git_runs
    .iter()
    .filter(|r| r.program == "git" && r.args.first().map(String::as_str) == Some("push"))
    .map(|r| {
      let args: Vec<&str> = r.args.iter().map(String::as_str).collect();
      format!("git {}", args.join(" "))
    })
    .collect();

  let mut verdicts: Vec<(String, FileVerdict)> = outcome
    .bump
    .verdicts
    .iter()
    .map(|(abs, verdict)| (crate::display::path(cwd, Path::new(abs)), *verdict))
    .collect();
  // 判定行排序（收集清单为排序序；missing 补行附尾后整体重排为路径序——
  // 计划按文件名字典序逐行预演，与收集序一致）
  verdicts.sort_by(|a, b| a.0.cmp(&b.0));
  let writes: Vec<PathBuf> = recorder
    .writes()
    .into_iter()
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect();

  Ok(BumpPlan {
    verdicts,
    current_version: outcome.state.current_version,
    current_version_source: outcome.state.current_version_source,
    new_version: outcome.state.new_version,
    writes,
    execute,
    installs,
    scripts,
    commit_message: outcome.bump.commit,
    tag_name: outcome.bump.tag,
    pushes,
    changelog: outcome.changelog.map(|c| c.markdown),
    release,
  })
}
