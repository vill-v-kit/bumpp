//! 候选版本计算：复刻上游 bumpp v11 `getNextVersion` / `getNextVersions` 语义。
//!
//! 递增算法对齐 node-semver `SemVer#inc`（identifierBase 缺省），叠加 bumpp 的
//! `0→1` 修正（仅当请求的 release type 为 pre* 且产出 `[preid, 0]` 时改为 1）。

use std::error::Error;
use std::fmt;

use semver::Version;

use crate::commits::{determine_semver_change, CommitInfo};

/// node-semver 数字段转 number 的阈值（Number.MAX_SAFE_INTEGER - 1）
const MAX_SAFE_INTEGER_EXCLUSIVE: u64 = 9007199254740991;

/// 发布类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseType {
  Premajor,
  Preminor,
  Prepatch,
  Prerelease,
  Major,
  Minor,
  Patch,
  Next,
  /// 依据约定式提交推断（需要 git 提交历史）
  Conventional,
}

impl ReleaseType {
  /// 上游 `releaseTypes` 顺序
  pub const ALL: [ReleaseType; 9] = [
    Self::Premajor,
    Self::Preminor,
    Self::Prepatch,
    Self::Prerelease,
    Self::Major,
    Self::Minor,
    Self::Patch,
    Self::Next,
    Self::Conventional,
  ];

  fn is_pre(self) -> bool {
    matches!(
      self,
      Self::Premajor | Self::Preminor | Self::Prepatch | Self::Prerelease
    )
  }
}

#[derive(Debug)]
pub enum VersionError {
  InvalidVersion(String),
  InvalidPreid(String),
}

impl fmt::Display for VersionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidVersion(v) => write!(f, "无效的版本号：{v}"),
      Self::InvalidPreid(p) => write!(
        f,
        "无效的 preid：{p:?}（须为单个标识符，[0-9A-Za-z-] 组成，数字段不得有前导零）"
      ),
    }
  }
}

impl Error for VersionError {}

/// 全部 release type 的候选版本（上游 `getNextVersions` 的返回形状）
#[derive(Debug, PartialEq, Eq)]
pub struct NextVersions {
  pub premajor: String,
  pub preminor: String,
  pub prepatch: String,
  pub prerelease: String,
  pub major: String,
  pub minor: String,
  pub patch: String,
  pub next: String,
  pub conventional: String,
}

impl NextVersions {
  pub fn get(&self, release: ReleaseType) -> &str {
    match release {
      ReleaseType::Premajor => &self.premajor,
      ReleaseType::Preminor => &self.preminor,
      ReleaseType::Prepatch => &self.prepatch,
      ReleaseType::Prerelease => &self.prerelease,
      ReleaseType::Major => &self.major,
      ReleaseType::Minor => &self.minor,
      ReleaseType::Patch => &self.patch,
      ReleaseType::Next => &self.next,
      ReleaseType::Conventional => &self.conventional,
    }
  }
}

/// 预发行段（node-semver 中数字段转 number、其余保持 string，递增只找数字段）
#[derive(Debug, Clone, PartialEq, Eq)]
enum PreSegment {
  Num(u64),
  Str(String),
}

impl PreSegment {
  fn parse(segment: &str) -> PreSegment {
    // node-semver：纯数字且 < MAX_SAFE_INTEGER 转 number，否则保持 string
    if segment.bytes().all(|b| b.is_ascii_digit()) {
      match segment.parse::<u64>() {
        Ok(n) if n < MAX_SAFE_INTEGER_EXCLUSIVE => PreSegment::Num(n),
        _ => PreSegment::Str(segment.to_owned()),
      }
    } else {
      PreSegment::Str(segment.to_owned())
    }
  }

  /// node-semver `compareIdentifiers`：两侧皆数字时按数值比较，否则按字符串
  fn matches_preid(&self, preid: &str) -> bool {
    match self {
      PreSegment::Num(n) => {
        !preid.is_empty()
          && preid.bytes().all(|b| b.is_ascii_digit())
          && preid.parse::<u64>().is_ok_and(|v| v == *n)
      }
      PreSegment::Str(s) => s == preid,
    }
  }
}

impl fmt::Display for PreSegment {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PreSegment::Num(n) => write!(f, "{n}"),
      PreSegment::Str(s) => f.write_str(s),
    }
  }
}

fn parse_pre(version: &Version) -> Vec<PreSegment> {
  if version.pre.is_empty() {
    Vec::new()
  } else {
    version.pre.split('.').map(PreSegment::parse).collect()
  }
}

fn format_version(major: u64, minor: u64, patch: u64, pre: &[PreSegment]) -> String {
  let base = format!("{major}.{minor}.{patch}");
  if pre.is_empty() {
    base
  } else {
    let joined = pre
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>()
      .join(".");
    format!("{base}-{joined}")
  }
}

/// 校验 preid，对齐 node-semver `inc` 对 identifier 的校验：
/// 单个标识符、`[0-9A-Za-z-]` 组成、纯数字时不得有前导零。
fn validate_preid(preid: &str) -> Result<(), VersionError> {
  let valid = !preid.is_empty()
    && preid
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    && !(preid.len() > 1 && preid.bytes().all(|b| b.is_ascii_digit()) && preid.starts_with('0'));
  if valid {
    Ok(())
  } else {
    Err(VersionError::InvalidPreid(preid.to_owned()))
  }
}

/// node-semver `SemVer#inc`（identifierBase 缺省）。
/// 返回递增后的 (major, minor, patch, prerelease)；build metadata 随 inc 丢弃。
fn inc(
  version: &Version,
  release: ReleaseType,
  preid: Option<&str>,
) -> (u64, u64, u64, Vec<PreSegment>) {
  let (mut major, mut minor, mut patch) = (version.major, version.minor, version.patch);
  let mut pre = parse_pre(version);

  match release {
    ReleaseType::Major => {
      if minor != 0 || patch != 0 || pre.is_empty() {
        major += 1;
      }
      minor = 0;
      patch = 0;
      pre.clear();
    }
    ReleaseType::Minor => {
      if patch != 0 || pre.is_empty() {
        minor += 1;
      }
      patch = 0;
      pre.clear();
    }
    ReleaseType::Patch => {
      if pre.is_empty() {
        patch += 1;
      }
      pre.clear();
    }
    ReleaseType::Premajor => {
      pre.clear();
      patch = 0;
      minor = 0;
      major += 1;
      inc_pre(&mut pre, preid);
    }
    ReleaseType::Preminor => {
      pre.clear();
      patch = 0;
      minor += 1;
      inc_pre(&mut pre, preid);
    }
    ReleaseType::Prepatch => {
      pre.clear();
      patch += 1;
      inc_pre(&mut pre, preid);
    }
    ReleaseType::Prerelease => {
      if pre.is_empty() {
        patch += 1;
      }
      inc_pre(&mut pre, preid);
    }
    ReleaseType::Next | ReleaseType::Conventional => {
      unreachable!("next/conventional 在调用处解析为具体类型")
    }
  }

  (major, minor, patch, pre)
}

/// node-semver `inc('pre', identifier)`：递增最右数字段（无则补 0），再按 preid 归位
fn inc_pre(pre: &mut Vec<PreSegment>, preid: Option<&str>) {
  if pre.is_empty() {
    pre.push(PreSegment::Num(0));
  } else {
    let mut incremented = false;
    for seg in pre.iter_mut().rev() {
      if let PreSegment::Num(n) = seg {
        *n += 1;
        incremented = true;
        break;
      }
    }
    if !incremented {
      pre.push(PreSegment::Num(0));
    }
  }

  if let Some(id) = preid {
    if pre.first().is_some_and(|seg| seg.matches_preid(id)) {
      if !matches!(pre.get(1), Some(PreSegment::Num(_))) {
        *pre = vec![PreSegment::Str(id.to_owned()), PreSegment::Num(0)];
      }
    } else {
      *pre = vec![PreSegment::Str(id.to_owned()), PreSegment::Num(0)];
    }
  }
}

/// 上游 `getNextVersion`：计算指定 release type 的下一版本。
/// `preid` 对应上游入参（未做 `getNextVersions` 的沿用处理）。
pub fn next_version(
  current: &str,
  release: ReleaseType,
  preid: Option<&str>,
  commits: &[CommitInfo],
) -> Result<String, VersionError> {
  let version =
    Version::parse(current).map_err(|_| VersionError::InvalidVersion(current.to_owned()))?;

  // node-semver 中空字符串 identifier 按未传入处理（falsy）
  let preid = preid.filter(|p| !p.is_empty());

  // 上游：next = 预发行中→prerelease、否则 patch；
  // conventional = 预发行中→prerelease、否则按提交推断（0→1 修正只看请求的 type）
  let resolved = match release {
    ReleaseType::Next if version.pre.is_empty() => ReleaseType::Patch,
    ReleaseType::Next => ReleaseType::Prerelease,
    ReleaseType::Conventional if version.pre.is_empty() => determine_semver_change(commits),
    ReleaseType::Conventional => ReleaseType::Prerelease,
    other => other,
  };

  if resolved.is_pre() {
    if let Some(p) = preid {
      validate_preid(p)?;
    }
  }

  let (major, minor, patch, mut pre) = inc(&version, resolved, preid);

  // bumpp 0→1 修正：仅当请求的 release type 为 pre* 且产出 [preid, 0]
  if release.is_pre()
    && pre.len() == 2
    && preid.is_some_and(|p| matches!(&pre[0], PreSegment::Str(s) if s == p))
    && pre[1] == PreSegment::Num(0)
  {
    pre[1] = PreSegment::Num(1);
  }

  Ok(format_version(major, minor, patch, &pre))
}

/// 上游 `getNextVersions`：全部 release type 的候选版本。
/// preid 沿用规则：当前版本预发行首段为字符串时沿用之，否则用入参。
pub fn next_versions(
  current: &str,
  preid: Option<&str>,
  commits: &[CommitInfo],
) -> Result<NextVersions, VersionError> {
  let version =
    Version::parse(current).map_err(|_| VersionError::InvalidVersion(current.to_owned()))?;

  let pre = parse_pre(&version);
  let inherited;
  let preid = match pre.first() {
    Some(PreSegment::Str(s)) => {
      inherited = s.as_str();
      Some(inherited)
    }
    _ => preid,
  };

  Ok(NextVersions {
    premajor: next_version(current, ReleaseType::Premajor, preid, commits)?,
    preminor: next_version(current, ReleaseType::Preminor, preid, commits)?,
    prepatch: next_version(current, ReleaseType::Prepatch, preid, commits)?,
    prerelease: next_version(current, ReleaseType::Prerelease, preid, commits)?,
    major: next_version(current, ReleaseType::Major, preid, commits)?,
    minor: next_version(current, ReleaseType::Minor, preid, commits)?,
    patch: next_version(current, ReleaseType::Patch, preid, commits)?,
    next: next_version(current, ReleaseType::Next, preid, commits)?,
    conventional: next_version(current, ReleaseType::Conventional, preid, commits)?,
  })
}
