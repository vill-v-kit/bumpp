//! git 操作：shell out 到 `git` 二进制（继承用户 git config / SSH / GPG / credential helper）。
//! 写操作（commit / tag / push）对齐上游 bumpp v11；只读历史操作（tag / diff / branch /
//! remote 解析）对齐 changelogen 0.6.2 同名函数（ADR-0012）。

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::exec::{capture, run, ExecError};
use crate::progress::ProgressEvent;

/// git commit 的输入参数（对齐上游 options.commit + operation.state）
pub struct CommitSpec<'a> {
  /// 已更新的文件（`all: false` 时按路径逐个提交）
  pub updated_files: &'a [String],
  pub all: bool,
  pub no_verify: bool,
  pub sign: bool,
  /// commit 信息模板（`%s` 替换为新版本号，无占位符则追加版本号）
  pub message: &'a str,
  pub new_version: &'a str,
}

/// git tag 的输入参数（对齐上游 options.tag + options.commit.message）
pub struct TagSpec<'a> {
  /// tag 名模板（同样支持 `%s`）
  pub name: &'a str,
  /// 附注信息模板（上游复用 commit.message）
  pub message: &'a str,
  pub sign: bool,
  pub new_version: &'a str,
}

/// 上游 `formatVersionString`：含 `%s` 则全部替换，否则在末尾追加版本号
pub fn format_version_string(template: &str, new_version: &str) -> String {
  if template.contains("%s") {
    template.replace("%s", new_version)
  } else {
    format!("{template}{new_version}")
  }
}

/// 上游 `gitCommit`：`--allow-empty [--all] [--no-verify] [--gpg-sign] --message <msg> [files...]`
pub fn git_commit(cwd: &Path, spec: &CommitSpec) -> Result<(ProgressEvent, String), ExecError> {
  let mut args = vec!["commit".to_string(), "--allow-empty".to_string()];
  if spec.all {
    args.push("--all".to_string());
  }
  if spec.no_verify {
    args.push("--no-verify".to_string());
  }
  if spec.sign {
    args.push("--gpg-sign".to_string());
  }
  let message = format_version_string(spec.message, spec.new_version);
  args.push("--message".to_string());
  args.push(message.clone());
  if !spec.all {
    args.extend(spec.updated_files.iter().cloned());
  }
  run("git", &args, cwd)?;
  Ok((ProgressEvent::GitCommit, message))
}

/// changelogen `getLastGitTag`：`git describe --tags --abbrev=0`（取末行）。
/// 软失败语义对齐 changelogen 的 try/catch → undefined：无 tag、非 git 仓库等
/// git 非零退出返回 `Ok(None)`（本函数是「有 tag 才生成 changelog」的探测位）；
/// git 二进制缺失等 spawn 失败仍报错传播
pub fn get_last_git_tag(cwd: &Path) -> Result<Option<String>, ExecError> {
  let args = vec![
    "describe".to_string(),
    "--tags".to_string(),
    "--abbrev=0".to_string(),
  ];
  match capture("git", &args, cwd) {
    Ok(output) => {
      let stdout = String::from_utf8_lossy(&output.stdout);
      Ok(stdout.trim().lines().last().map(str::to_owned))
    }
    Err(ExecError::Failed { .. }) => Ok(None),
    Err(other) => Err(other),
  }
}

/// changelogen `getCurrentGitBranch`：`git rev-parse --abbrev-ref HEAD`。
/// 无 changelogen 侧的 catch——非 git 仓库等失败报错传播（detached HEAD 得 "HEAD"）
pub fn get_current_git_branch(cwd: &Path) -> Result<String, ExecError> {
  let args = vec![
    "rev-parse".to_string(),
    "--abbrev-ref".to_string(),
    "HEAD".to_string(),
  ];
  let output = capture("git", &args, cwd)?;
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// changelogen `RawCommit` 的 author 字段
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitAuthor {
  pub name: String,
  pub email: String,
}

/// changelogen `RawCommit`（`getGitDiff` 的元素）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCommit {
  pub message: String,
  pub short_hash: String,
  pub author: GitAuthor,
  /// 提交正文（`%b`）与 `--name-status` 行——对齐 changelogen 原解析，
  /// 展示层（ADR-0012）自行取舍
  pub body: String,
}

/// changelogen `getGitDiff`：
/// `git --no-pager log "<from>...<to>" --pretty="----%n%s|%h|%an|%ae%n%b" --name-status`
/// 三点对称差范围（线性史上等价两点）；`from` 为空取 `<to>` 全史；`to` 缺省 HEAD；
/// 顺序保持 git log 默认（新→旧）
pub fn get_git_diff(cwd: &Path, from: &str, to: Option<&str>) -> Result<Vec<RawCommit>, ExecError> {
  let to = to.unwrap_or("HEAD");
  let range = if from.is_empty() {
    to.to_string()
  } else {
    format!("{from}...{to}")
  };
  let args = vec![
    "--no-pager".to_string(),
    "log".to_string(),
    range,
    "--pretty=----%n%s|%h|%an|%ae%n%b".to_string(),
    "--name-status".to_string(),
  ];
  let output = capture("git", &args, cwd)?;
  let stdout = String::from_utf8_lossy(&output.stdout);
  Ok(parse_git_log(stdout.trim()))
}

/// changelogen 原解析：`split("----\n").splice(1)`，首行 `%s|%h|%an|%ae`，余为 body。
/// 首行切分对齐 JS `split("|")` 后解构前四段（多余段丢弃）——主题含 `|` 时
/// 字段位移的病理行为也逐字保留，不擅自修复
fn parse_git_log(stdout: &str) -> Vec<RawCommit> {
  stdout
    .split("----\n")
    .skip(1)
    .map(|chunk| {
      let mut lines = chunk.lines();
      let first = lines.next().unwrap_or("");
      let segments: Vec<&str> = first.split('|').collect();
      let segment = |i: usize| segments.get(i).unwrap_or(&"").to_string();
      RawCommit {
        message: segment(0),
        short_hash: segment(1),
        author: GitAuthor {
          name: segment(2),
          email: segment(3),
        },
        body: lines.collect::<Vec<_>>().join("\n"),
      }
    })
    .collect()
}

/// changelogen `RepoConfig`：远程仓库解析结果（provider / domain / repo 三元组，
/// 各字段视解析形态可为 None）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoConfig {
  pub provider: Option<String>,
  pub domain: Option<String>,
  pub repo: Option<String>,
}

/// changelogen `providerURLRegex`：scp-like（`git@host:owner/repo.git`）与裸
/// `owner/repo` 的形态匹配。原正则的负向断言 `\.(?!git$)`（Rust regex 不支持）
/// 以 lazy 匹配 + 尾部 `(?:\.git)?` 可选组等价：最短匹配优先让位给 .git 后缀；
/// 唯一的残余差异（第二段恰为 `.git`，如 `owner/.git`）在 get_repo_config 内显式拒绝
static PROVIDER_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^(?:[A-Za-z0-9_-]+@)?(?:([^/:]+):)?([A-Za-z0-9_-]+/[A-Za-z0-9_.-]*?)(?:\.git)?$")
    .unwrap()
});

/// changelogen `providerToDomain` / `domainToProvider`（仅识别三个 canonical 域名）
const PROVIDER_TO_DOMAIN: [(&str, &str); 3] = [
  ("github", "github.com"),
  ("gitlab", "gitlab.com"),
  ("bitbucket", "bitbucket.org"),
];

fn domain_to_provider(domain: &str) -> Option<&'static str> {
  PROVIDER_TO_DOMAIN
    .iter()
    .find(|(_, d)| *d == domain)
    .map(|(p, _)| *p)
}

fn provider_to_domain(provider: &str) -> Option<&'static str> {
  PROVIDER_TO_DOMAIN
    .iter()
    .find(|(p, _)| *p == provider)
    .map(|(_, d)| *d)
}

/// changelogen `getRepoConfig`：scp-like 正则分支 → URL 分支 → 裸 owner/repo 缺省
/// github 分支；均不命中返回全 None（对齐 changelogen 的全 undefined）
pub fn get_repo_config(repo_url: &str) -> RepoConfig {
  let caps = PROVIDER_URL_RE.captures(repo_url);
  let (re_provider, re_repo) = match &caps {
    Some(c) => (c.get(1).map(|m| m.as_str()), c.get(2).map(|m| m.as_str())),
    None => (None, None),
  };
  // 原正则的残余差异两则（lazy 等价转换的边角）：第二段恰为 `.git`（`owner/.git`）
  // 与空第二段（`owner/`）——changelogen 的 `+` 量词与负向断言均整体不匹配
  let re_repo = re_repo.filter(|r| !r.ends_with('/') && !r.ends_with("/.git"));
  if let (Some(provider_raw), Some(repo)) = (re_provider, re_repo) {
    let provider = domain_to_provider(provider_raw).unwrap_or(provider_raw);
    let domain = provider_to_domain(provider).unwrap_or(provider);
    return RepoConfig {
      provider: Some(provider.to_owned()),
      domain: Some(domain.to_owned()),
      repo: Some(repo.to_owned()),
    };
  }
  if let Ok(url) = url::Url::parse(repo_url) {
    let path = url.path().trim_start_matches('/');
    let repo = path.strip_suffix(".git").unwrap_or(path);
    return RepoConfig {
      provider: url
        .host_str()
        .and_then(domain_to_provider)
        .map(str::to_owned),
      domain: url.host_str().map(str::to_owned),
      repo: Some(repo.to_owned()),
    };
  }
  if let Some(repo) = re_repo {
    return RepoConfig {
      provider: Some("github".to_owned()),
      domain: Some("github.com".to_owned()),
      repo: Some(repo.to_owned()),
    };
  }
  RepoConfig {
    provider: None,
    domain: None,
    repo: None,
  }
}

/// changelogen `resolveRepoConfig`：package.json `repository` 键优先（truthiness
/// 语义见 read_package_repository），`git remote get-url origin` 兜底；两源皆无
/// 返回 None（changelogen 的 undefined）。兜底分支的 `.ok()?` 吞掉全部执行失败
/// （含 spawn）——对齐 changelogen 该分支的 try/catch，与 get_last_git_tag 刻意
/// 区分 Spawn/Failed 不同，此处是「无 remote 可用」探测
pub fn resolve_repo_config(cwd: &Path) -> Option<RepoConfig> {
  if let Some(repository) = read_package_repository(cwd) {
    return Some(get_repo_config(&repository));
  }
  let args = vec![
    "remote".to_string(),
    "get-url".to_string(),
    "origin".to_string(),
  ];
  let output = capture("git", &args, cwd).ok()?;
  let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_owned();
  if remote_url.is_empty() {
    return None;
  }
  Some(get_repo_config(&remote_url))
}

/// 读取 package.json 的 `repository` 键为 URL 字符串，对齐 changelogen 的
/// truthiness 语义：键存在且为真值即短路（string 直取，object 取 `url` 字段，
/// 其余形态或 object 无 url 均以空串短路为全 None 配置）；falsy（`""` / `false` /
/// `0` / `null` 等）落到 git remote 分支；文件缺失/不可解析/无该键同落 remote
fn read_package_repository(cwd: &Path) -> Option<String> {
  let content = std::fs::read_to_string(cwd.join("package.json")).ok()?;
  let value: serde_json::Value = serde_json::from_str(&content).ok()?;
  let repository = value.get("repository")?;
  if !js_truthy(repository) {
    return None;
  }
  Some(match repository {
    serde_json::Value::String(url) => url.clone(),
    serde_json::Value::Object(map) => map
      .get("url")
      .and_then(|u| u.as_str())
      .unwrap_or("")
      .to_owned(),
    _ => String::new(),
  })
}

/// JS truthiness：`""` / `false` / `0` / `null`（及缺失）为 falsy，余为 truthy
fn js_truthy(value: &serde_json::Value) -> bool {
  match value {
    serde_json::Value::Null => false,
    serde_json::Value::Bool(b) => *b,
    serde_json::Value::Number(n) => n.as_f64().is_none_or(|f| f != 0.0),
    serde_json::Value::String(s) => !s.is_empty(),
    _ => true,
  }
}

/// 上游 `gitTag`：`--annotate --message <commit.message 格式化> <tagName> [--sign]`
/// 注意：git tag 没有 hooks，上游不加 --no-verify
pub fn git_tag(cwd: &Path, spec: &TagSpec) -> Result<(ProgressEvent, String), ExecError> {
  let tag_name = format_version_string(spec.name, spec.new_version);
  let mut args = vec![
    "tag".to_string(),
    "--annotate".to_string(),
    "--message".to_string(),
    format_version_string(spec.message, spec.new_version),
    tag_name.clone(),
  ];
  if spec.sign {
    args.push("--sign".to_string());
  }
  run("git", &args, cwd)?;
  Ok((ProgressEvent::GitTag, tag_name))
}

/// 上游 `gitPush`：`git push`，启用 tag 时追加 `git push --tags`
pub fn git_push(cwd: &Path, with_tags: bool) -> Result<ProgressEvent, ExecError> {
  run("git", &["push".to_string()], cwd)?;
  if with_tags {
    run("git", &["push".to_string(), "--tags".to_string()], cwd)?;
  }
  Ok(ProgressEvent::GitPush)
}
