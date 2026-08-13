//! bumpVersion 编排（COL-60）：配置文件 `release` 键接通非交互路径——
//! 跳过版本菜单直接按配置发版（CI / 脚本化场景）；`confirm` 在非交互路径
//! 生效（缺省 true，执行前二次确认），交互选定版本后不再问（文档语义）。

mod common;

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vbumpp_core::orchestrate::{
  bump_version, BumpVersionOptions, BumpVersionOutcome, OrchestrateError,
};

fn bump(path: &Path) -> Result<BumpVersionOutcome, OrchestrateError> {
  common::isolate_global_home();
  bump_version(&BumpVersionOptions::default(), path)
}

#[test]
fn config_release_skips_prompt_and_completes_bump() {
  // COL-60 主场景：release + confirm=false 全程非交互——测试环境非 TTY，
  // 任何交互 prompt 都会报错，跑通即证明零交互
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\nconfirm = false\npush = false\n");
  let outcome = bump(&path).unwrap();
  assert_eq!(outcome.state.current_version, "1.0.0");
  assert_eq!(outcome.state.new_version, "1.1.0");
  assert_eq!(outcome.state.release.as_deref(), Some("minor"));
  let pkg = fs::read_to_string(path.join("package.json")).unwrap();
  assert!(pkg.contains("1.1.0"), "package.json 应已更新：{pkg}");
  common::git(&path, &["rev-parse", "--verify", "refs/tags/v1.1.0"]);
  assert!(outcome.changelog.is_some(), "有 tag 应生成 changelog");
  assert!(path.join("CHANGELOG.md").is_file());
}

#[test]
fn config_release_version_string_bumps_exactly() {
  // release 直接给版本号（上游 loose 解析：v2.0 → 2.0.0）
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"v2.0\"\nconfirm = false\npush = false\n");
  let outcome = bump(&path).unwrap();
  assert_eq!(outcome.state.new_version, "2.0.0");
  assert_eq!(outcome.state.release, None, "version 路径 release 为空");
}

#[test]
fn config_release_honors_confirm_default_with_second_confirmation() {
  // 非交互 + confirm 缺省 true（上游语义）：执行前二次确认。非 TTY 测试
  // 环境下 dialoguer 确认框必报错——借此证明 confirm 在非交互路径已被消费
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"patch\"\npush = false\n");
  let err = bump(&path).unwrap_err();
  match err {
    OrchestrateError::Bump { message } => {
      assert!(
        message.contains("confirmation prompt failed"),
        "应死于确认框（confirm 生效）：{message}"
      );
    }
    other => panic!("应为 Bump（确认框失败），实际 {other:?}"),
  }
}

#[test]
fn config_preid_shapes_prerelease() {
  // preid 键与 release 同线接通（缺它 pre* 释放算错标识）
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(
    &dir,
    "release = \"preminor\"\npreid = \"rc\"\nconfirm = false\npush = false\n",
  );
  let outcome = bump(&path).unwrap();
  assert_eq!(outcome.state.new_version, "1.1.0-rc.1");
}

#[test]
fn config_current_version_overrides_detection() {
  // currentVersion 键与 release 同线接通（缺它新版本基线仍来自文件探测）
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(
    &dir,
    "release = \"minor\"\ncurrentVersion = \"9.9.9\"\nconfirm = false\npush = false\n",
  );
  let outcome = bump(&path).unwrap();
  assert_eq!(outcome.state.current_version, "9.9.9");
  assert_eq!(outcome.state.new_version, "9.10.0");
}

#[test]
fn config_release_non_string_errors() {
  // release = 1 这类错误类型不允许静默回落交互（防配置静默失效），明确报错
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = 1\nconfirm = false\npush = false\n");
  let err = bump(&path).unwrap_err();
  match err {
    OrchestrateError::Config { message } => {
      assert!(message.contains("release"), "应指出是哪个键：{message}");
    }
    other => panic!("应为 Config，实际 {other:?}"),
  }
}

#[test]
fn config_release_empty_string_errors() {
  // release = "" 同样不允许静默当缺省（否则意外弹交互菜单）——上游
  // `release: ""` 亦经 loose 解析抛错
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"\"\nconfirm = false\npush = false\n");
  let err = bump(&path).unwrap_err();
  match err {
    OrchestrateError::Config { message } => {
      assert!(message.contains("release"), "应指出是哪个键：{message}");
      assert!(message.contains("non-empty string"), "{message}");
    }
    other => panic!("应为 Config，实际 {other:?}"),
  }
}

#[test]
fn config_release_invalid_value_surfaces_invalid_version() {
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(
    &dir,
    "release = \"banana\"\nconfirm = false\npush = false\n",
  );
  let err = bump(&path).unwrap_err();
  match err {
    OrchestrateError::Info { message } => {
      assert!(message.contains("invalid version"), "{message}");
      assert!(message.contains("banana"), "{message}");
    }
    other => panic!("应为 Info，实际 {other:?}"),
  }
}

#[test]
fn no_release_key_keeps_interactive_menu() {
  // 回归锚定：不配 release 仍走版本菜单（非 TTY 下被守卫拦截为证）
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "confirm = false\npush = false\n");
  let err = bump(&path).unwrap_err();
  match err {
    OrchestrateError::Info { message } => {
      assert!(
        message.contains("requires a terminal"),
        "应死于版本菜单（仍交互）：{message}"
      );
    }
    other => panic!("应为 Info（交互菜单失败），实际 {other:?}"),
  }
}

#[test]
fn config_release_prompt_value_forces_interactive_menu() {
  // release = "prompt" 显式强制交互（info.rs 既有语义），且选定后不再问
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(
    &dir,
    "release = \"prompt\"\nconfirm = false\npush = false\n",
  );
  let err = bump(&path).unwrap_err();
  match err {
    OrchestrateError::Info { message } => {
      assert!(message.contains("requires a terminal"), "{message}");
    }
    other => panic!("应为 Info（交互菜单失败），实际 {other:?}"),
  }
}

/// 上游 parity 钉定（COL-85 透传缝口）：versionBumpInfo 的候选文件清单 =
/// options.files（收集前）——默认探测表（链上 manifest basenames 并集）
/// 在显式清单为空时照常兜底，来源为探测到的清单文件名
#[test]
fn default_probe_table_supplies_current_version_source() {
  common::isolate_global_home();
  let dir = TempDir::new().unwrap();
  let path = common::init_bump_repo(&dir, "release = \"minor\"\npush = false\n");

  let options = BumpVersionOptions {
    overrides: Some(
      serde_json::json!({
        "recursive": false,
        "changelog": { "output": "CHANGELOG.md" },
        "confirm": false
      })
      .as_object()
      .unwrap()
      .clone(),
    ),
    provider: None,
  };
  let outcome = bump_version(&options, &path).unwrap();
  assert_eq!(outcome.state.current_version, "1.0.0");
  assert_eq!(
    outcome.state.current_version_source, "package.json",
    "默认探测表兜底：来源为探测到的清单文件名"
  );
}
