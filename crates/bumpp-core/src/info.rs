//! versionBumpInfo 全链路：读取当前版本 → 计算候选版本 →（必要时）prompt。
//! 对齐上游 bumpp v11 `versionBumpInfo` / `getNewVersion`。

use std::error::Error;
use std::fmt;
use std::path::Path;

use semver::Version;

use crate::commits::get_recent_commits;
use crate::prompt::prompt_new_version;
use crate::version::{next_version, next_versions, ReleaseType};

/// versionBumpInfo 的输入（上游 VersionBumpOptions 的相关子集）
pub struct BumpInfoOptions<'a> {
  /// release type 或版本号；None / "prompt" 走交互 prompt
  pub release: Option<&'a str>,
  /// 扫描当前版本的候选文件（按序先命中先赢；清单外追加链上 basename 探测表，ADR-0009）
  pub files: &'a [String],
  /// 显式指定当前版本（跳过文件扫描）
  pub current_version: Option<&'a str>,
  pub preid: Option<&'a str>,
}

/// 上游 operation.state 的形状
#[derive(Debug, PartialEq, Eq)]
pub struct BumpState {
  pub release: Option<String>,
  pub current_version: String,
  pub current_version_source: String,
  pub new_version: String,
  pub commit_message: String,
  pub tag_name: String,
  pub updated_files: Vec<String>,
  pub skipped_files: Vec<String>,
}

impl BumpState {
  pub(crate) fn new(current_version: String, current_version_source: String) -> Self {
    Self {
      release: None,
      current_version,
      current_version_source,
      new_version: String::new(),
      commit_message: String::new(),
      tag_name: String::new(),
      updated_files: Vec::new(),
      skipped_files: Vec::new(),
    }
  }
}

#[derive(Debug)]
pub enum InfoError {
  UnableToDetermineVersion { message: String },
  InvalidVersion { message: String },
  Prompt { message: String },
}

impl fmt::Display for InfoError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnableToDetermineVersion { message }
      | Self::InvalidVersion { message }
      | Self::Prompt { message } => f.write_str(message),
    }
  }
}

impl Error for InfoError {}

/// 上游 `versionBumpInfo`：start → getRecentCommits → getCurrentVersion → getNewVersion
pub fn version_bump_info(options: &BumpInfoOptions, cwd: &Path) -> Result<BumpState, InfoError> {
  let commits = get_recent_commits(cwd, None, None);
  let (current_version, source) = get_current_version(options.files, options.current_version, cwd)?;
  let mut state = BumpState::new(current_version, source);

  // 上游 normalizeOptions：preid 缺省为 "beta"
  let (release, new_version) = resolve_new_version(
    options.release,
    options.preid,
    &state.current_version,
    &commits,
  )?;
  state.release = release;
  state.new_version = new_version;
  Ok(state)
}

/// 上游 `getNewVersion`：prompt / release type / loose 版本号三路解析。
/// 返回 (state.release, newVersion)。
pub(crate) fn resolve_new_version(
  release: Option<&str>,
  preid: Option<&str>,
  current_version: &str,
  commits: &[crate::commits::CommitInfo],
) -> Result<(Option<String>, String), InfoError> {
  // 上游 normalizeOptions：preid 缺省为 "beta"
  let preid = preid.or(Some("beta"));
  match release {
    None | Some("prompt") => {
      // 上游：getNextVersions(currentVersion, release.preid, commits) 后 prompt
      let next =
        next_versions(current_version, preid, commits).map_err(|e| InfoError::InvalidVersion {
          message: e.to_string(),
        })?;
      prompt_new_version(current_version, &next)
    }
    Some(raw) => match ReleaseType::parse(raw) {
      Some(release) => {
        let new_version = next_version(current_version, release, preid, commits).map_err(|e| {
          InfoError::InvalidVersion {
            message: e.to_string(),
          }
        })?;
        Ok((Some(raw.to_string()), new_version))
      }
      None => {
        // 上游 case "version"：new SemVer(release.version, true)（loose 解析）
        Ok((None, parse_loose(raw)?))
      }
    },
  }
}

/// node-semver 的 loose 解析子集：去 v/=/空白前缀，补齐缺失的 minor/patch
fn parse_loose(raw: &str) -> Result<String, InfoError> {
  let cleaned = raw.trim().trim_start_matches(['=', 'v', 'V', ' ']);
  let mut parts: Vec<&str> = cleaned.split('.').collect();
  while parts.len() < 3 {
    parts.push("0");
  }
  Version::parse(&parts.join("."))
    .map(|v| v.to_string())
    .map_err(|_| InfoError::InvalidVersion {
      message: format!("无效的版本号：{raw}"),
    })
}

/// 上游 `getCurrentVersion`：options.currentVersion 优先，否则按候选文件顺序
/// 经插件底座链分发读取（ADR-0009）；候选清单之外追加探测表——链上清单
/// basename 并集（node 8 项在 cargo.toml 前，链序即优先级）
pub(crate) fn get_current_version(
  files: &[String],
  current_version: Option<&str>,
  cwd: &Path,
) -> Result<(String, String), InfoError> {
  if let Some(v) = current_version {
    // 上游 Operation 构造器：显式 currentVersion 时 currentVersionSource 为 "user"
    return Ok((v.to_string(), "user".to_string()));
  }
  let mut files_to_check: Vec<String> = files.to_vec();
  for probe in crate::plugins::default_file_patterns(false) {
    if !files_to_check.contains(&probe) {
      files_to_check.push(probe);
    }
  }
  for file in &files_to_check {
    let abs = crate::plugins::resolve(cwd, file);
    if let Some(version) = crate::plugins::dispatch_read_version(Path::new(file), &abs) {
      return Ok((version, file.clone()));
    }
  }
  Err(InfoError::UnableToDetermineVersion {
    message: format!(
      "Unable to determine the current version number. Checked {}.",
      files_to_check.join(", ")
    ),
  })
}
