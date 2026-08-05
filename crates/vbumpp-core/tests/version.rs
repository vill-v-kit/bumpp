//! 候选版本计算矩阵——期望值由真实 node-semver@7.8.5 逐格生成（上游 bumpp v11
//! `getNextVersions` 的忠实复刻脚本输出），作为 parity spec。
//!
//! 顺序对齐上游 releaseTypes（去掉 conventional，由 COL-13 引入）：
//! [premajor, preminor, prepatch, prerelease, major, minor, patch, next]

use vbumpp_core::version::{next_version, next_versions, ReleaseType, VersionError};

fn check(current: &str, preid: Option<&str>, expected: [&str; 9]) {
  let next = next_versions(current, preid, &[]).unwrap();
  let actual: Vec<String> = ReleaseType::ALL
    .iter()
    .map(|t| next.get(*t).to_owned())
    .collect();
  assert_eq!(
    actual,
    expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    "current={current} preid={preid:?}"
  );
}

#[test]
fn next_version_does_not_inherit_preid() {
  // 分层语义：next_version 对齐上游 getNextVersion，直接使用入参 preid，
  // 沿用规则只存在于 next_versions（上游 getNextVersions）一层
  assert_eq!(
    next_version("1.0.0-beta.0", ReleaseType::Premajor, None, &[]).unwrap(),
    "2.0.0-0"
  );
  assert_eq!(
    next_version("1.0.0-beta.0", ReleaseType::Premajor, Some("beta"), &[]).unwrap(),
    "2.0.0-beta.1"
  );
  // next 的解析与 0→1 修正按请求类型生效
  assert_eq!(
    next_version("1.0.0-0", ReleaseType::Next, Some("preid"), &[]).unwrap(),
    "1.0.0-preid.0"
  );
  assert_eq!(
    next_version("1.0.0-0", ReleaseType::Prerelease, Some("preid"), &[]).unwrap(),
    "1.0.0-preid.1"
  );
}

#[test]
fn formal_versions_without_preid() {
  // preid 缺省（undefined）：pre* 产出数字 0 后缀，且不触发 0→1 修正
  check(
    "1.2.3",
    None,
    [
      "2.0.0-0", "1.3.0-0", "1.2.4-0", "1.2.4-0", "2.0.0", "1.3.0", "1.2.4", "1.2.4", "1.2.4",
    ],
  );
  check(
    "0.0.0",
    None,
    [
      "1.0.0-0", "0.1.0-0", "0.0.1-0", "0.0.1-0", "1.0.0", "0.1.0", "0.0.1", "0.0.1", "0.0.1",
    ],
  );
  check(
    "0.1.0",
    None,
    [
      "1.0.0-0", "0.2.0-0", "0.1.1-0", "0.1.1-0", "1.0.0", "0.2.0", "0.1.1", "0.1.1", "0.1.1",
    ],
  );
}

#[test]
fn formal_versions_with_preid() {
  // 字符串 preid：pre* 产出 .0 后被 0→1 修正为 .1
  check(
    "1.2.3",
    Some("preid"),
    [
      "2.0.0-preid.1",
      "1.3.0-preid.1",
      "1.2.4-preid.1",
      "1.2.4-preid.1",
      "2.0.0",
      "1.3.0",
      "1.2.4",
      "1.2.4",
      "1.2.4",
    ],
  );
  check(
    "0.0.0",
    Some("preid"),
    [
      "1.0.0-preid.1",
      "0.1.0-preid.1",
      "0.0.1-preid.1",
      "0.0.1-preid.1",
      "1.0.0",
      "0.1.0",
      "0.0.1",
      "0.0.1",
      "0.0.1",
    ],
  );
  check(
    "0.1.0",
    Some("preid"),
    [
      "1.0.0-preid.1",
      "0.2.0-preid.1",
      "0.1.1-preid.1",
      "0.1.1-preid.1",
      "1.0.0",
      "0.2.0",
      "0.1.1",
      "0.1.1",
      "0.1.1",
    ],
  );
  // 自定义 preid
  check(
    "1.2.3",
    Some("alpha"),
    [
      "2.0.0-alpha.1",
      "1.3.0-alpha.1",
      "1.2.4-alpha.1",
      "1.2.4-alpha.1",
      "2.0.0",
      "1.3.0",
      "1.2.4",
      "1.2.4",
      "1.2.4",
    ],
  );
}

#[test]
fn prerelease_versions_inherit_string_preid() {
  // 当前版本预发行标识为字符串时，无论入参 preid 为何都沿用之
  let expected = [
    "2.0.0-beta.1",
    "1.1.0-beta.1",
    "1.0.1-beta.1",
    "1.0.0-beta.1",
    "1.0.0",
    "1.0.0",
    "1.0.0",
    "1.0.0-beta.1",
    "1.0.0-beta.1",
  ];
  check("1.0.0-beta.0", None, expected);
  check("1.0.0-beta.0", Some("preid"), expected);

  let expected = [
    "2.0.0-beta.1",
    "1.3.0-beta.1",
    "1.2.4-beta.1",
    "1.2.3-beta.2",
    "2.0.0",
    "1.3.0",
    "1.2.3",
    "1.2.3-beta.2",
    "1.2.3-beta.2",
  ];
  check("1.2.3-beta.1", None, expected);
  check("1.2.3-beta.1", Some("preid"), expected);
}

#[test]
fn prerelease_with_zero_patch_bumps_in_place() {
  // major/minor/patch 在 minor/patch 已为 0 且处于预发行时只清除预发行
  let expected = [
    "3.0.0-alpha.1",
    "2.1.0-alpha.1",
    "2.0.1-alpha.1",
    "2.0.0-alpha.6",
    "2.0.0",
    "2.0.0",
    "2.0.0",
    "2.0.0-alpha.6",
    "2.0.0-alpha.6",
  ];
  check("2.0.0-alpha.5+deadbeef", None, expected);
  check("2.0.0-alpha.5+deadbeef", Some("preid"), expected);
}

#[test]
fn numeric_prerelease_does_not_inherit_preid() {
  // 预发行首段为数字（1.0.0-0）时不沿用 preid，沿用入参
  check(
    "1.0.0-0",
    None,
    [
      "2.0.0-0", "1.1.0-0", "1.0.1-0", "1.0.0-1", "1.0.0", "1.0.0", "1.0.0", "1.0.0-1", "1.0.0-1",
    ],
  );
  check(
    "1.0.0-0",
    Some("preid"),
    [
      "2.0.0-preid.1",
      "1.1.0-preid.1",
      "1.0.1-preid.1",
      "1.0.0-preid.1",
      "1.0.0",
      "1.0.0",
      "1.0.0",
      // next 解析为 prerelease 计算，但 0→1 修正只看请求的 release type，故保持 .0
      "1.0.0-preid.0",
      "1.0.0-preid.0",
    ],
  );
}

#[test]
fn build_metadata_is_dropped() {
  check(
    "1.2.3+build.5",
    None,
    [
      "2.0.0-0", "1.3.0-0", "1.2.4-0", "1.2.4-0", "2.0.0", "1.3.0", "1.2.4", "1.2.4", "1.2.4",
    ],
  );
  check(
    "1.2.3-beta.1+build.1",
    Some("preid"),
    [
      "2.0.0-beta.1",
      "1.3.0-beta.1",
      "1.2.4-beta.1",
      "1.2.3-beta.2",
      "2.0.0",
      "1.3.0",
      "1.2.3",
      "1.2.3-beta.2",
      "1.2.3-beta.2",
    ],
  );
}

#[test]
fn invalid_version_errors() {
  for bad in ["", "1.2", "v1.2.3", "1.2.3-", "not-a-version"] {
    assert!(
      matches!(
        next_versions(bad, None, &[]),
        Err(VersionError::InvalidVersion(_))
      ),
      "{bad:?} 应报 InvalidVersion"
    );
  }
}

#[test]
fn invalid_preid_errors() {
  // 带点、含非法字符、数字前导零的 preid 在 pre* 类型下报错（对齐 node-semver 校验）
  for bad in ["beta.x", "beta_1", "01"] {
    assert!(
      matches!(
        next_version("1.2.3", ReleaseType::Prepatch, Some(bad), &[]),
        Err(VersionError::InvalidPreid(_))
      ),
      "{bad:?} 应报 InvalidPreid"
    );
  }
  // 非 pre 类型不使用 preid，不校验
  assert!(next_version("1.2.3", ReleaseType::Patch, Some("beta.x"), &[]).is_ok());
}

#[test]
fn empty_preid_behaves_like_none() {
  // node-semver 中空字符串 identifier 是 falsy，按未传入处理
  assert_eq!(
    next_version("1.2.3", ReleaseType::Prepatch, Some(""), &[]).unwrap(),
    "1.2.4-0"
  );
  assert_eq!(
    next_version("1.0.0-beta.1", ReleaseType::Prerelease, Some(""), &[]).unwrap(),
    "1.0.0-beta.2"
  );
}

#[test]
fn huge_numeric_segments_stay_strings() {
  // node-semver：数字段 < MAX_SAFE_INTEGER 才转 number，否则保持 string
  assert_eq!(
    next_version(
      "1.0.0-99999999999999999999999",
      ReleaseType::Patch,
      None,
      &[]
    )
    .unwrap(),
    "1.0.0"
  );
  // MAX_SAFE_INTEGER 本身即为 string，无数字段可增则补 .0
  assert_eq!(
    next_version(
      "1.0.0-alpha.9007199254740991",
      ReleaseType::Prerelease,
      None,
      &[]
    )
    .unwrap(),
    "1.0.0-alpha.9007199254740991.0"
  );
  // 阈值之下按数字递增
  assert_eq!(
    next_version(
      "1.0.0-alpha.9007199254740990",
      ReleaseType::Prerelease,
      None,
      &[]
    )
    .unwrap(),
    "1.0.0-alpha.9007199254740991"
  );
}

#[test]
fn numeric_preid_compares_numerically() {
  // node-semver compareIdentifiers：两侧皆数字按数值比较
  assert_eq!(
    next_version("1.0.0-0.5", ReleaseType::Prerelease, Some("0"), &[]).unwrap(),
    "1.0.0-0.6"
  );
  // 数字 preid 无既有预发行段时归位为 [preid, 0]，再经 0→1 修正
  assert_eq!(
    next_version("1.2.3", ReleaseType::Prerelease, Some("0"), &[]).unwrap(),
    "1.2.4-0.1"
  );
}
