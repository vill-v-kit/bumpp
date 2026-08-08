//! versionBumpInfo 非交互路径与 prompt 选项文案——对齐上游 bumpp v11。

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use vbumpp_core::info::{version_bump_info, BumpInfoOptions, InfoError};
use vbumpp_core::prompt::build_choices;
use vbumpp_core::version::next_versions;

fn git(dir: &Path, args: &[&str]) {
  let status = Command::new("git")
    .args(args)
    .current_dir(dir)
    .output()
    .unwrap();
  assert!(status.status.success(), "git {args:?} 失败");
}

fn init_repo(dir: &TempDir) -> std::path::PathBuf {
  let path = dir.path().to_path_buf();
  git(&path, &["init", "-b", "main"]);
  git(&path, &["config", "user.email", "test@example.com"]);
  git(&path, &["config", "user.name", "Test"]);
  fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.2.3\"\n}\n",
  )
  .unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "chore: init"]);
  path
}

fn opts<'a>(release: Option<&'a str>) -> BumpInfoOptions<'a> {
  BumpInfoOptions {
    release,
    files: &[],
    current_version: None,
    preid: None,
  }
}

// ---- 非交互路径 ----

#[test]
fn release_type_major_computes_without_prompt() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let state = version_bump_info(&opts(Some("major")), &path).unwrap();
  assert_eq!(state.current_version, "1.2.3");
  assert_eq!(state.new_version, "2.0.0");
  assert_eq!(state.release.as_deref(), Some("major"));
  assert_eq!(state.current_version_source, "package.json");
}

#[test]
fn release_next_resolves_by_current_shape() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let state = version_bump_info(&opts(Some("next")), &path).unwrap();
  assert_eq!(state.new_version, "1.2.4");
  assert_eq!(state.release.as_deref(), Some("next"));
}

#[test]
fn release_conventional_infers_from_commits() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  fs::write(path.join("f"), "x").unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "feat: new thing"]);
  let state = version_bump_info(&opts(Some("conventional")), &path).unwrap();
  assert_eq!(state.new_version, "1.3.0");
  assert_eq!(state.release.as_deref(), Some("conventional"));
}

#[test]
fn release_version_string_is_loose_parsed() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let state = version_bump_info(&opts(Some("v2.0")), &path).unwrap();
  assert_eq!(state.new_version, "2.0.0");
  assert_eq!(state.release, None, "version 路径 release 为空");
}

#[test]
fn current_version_option_skips_file_scan() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let opts = BumpInfoOptions {
    release: Some("minor"),
    files: &[],
    current_version: Some("9.9.9"),
    preid: None,
  };
  let state = version_bump_info(&opts, &path).unwrap();
  assert_eq!(state.current_version, "9.9.9");
  assert_eq!(state.new_version, "9.10.0");
}

#[test]
fn current_version_scanned_from_deno_json() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  // 删掉 package.json，留下 deno.json
  fs::remove_file(path.join("package.json")).unwrap();
  fs::write(path.join("deno.json"), "{\n  \"version\": \"3.4.5\"\n}\n").unwrap();
  let state = version_bump_info(&opts(Some("patch")), &path).unwrap();
  assert_eq!(state.current_version, "3.4.5");
  assert_eq!(state.current_version_source, "deno.json");
}

#[test]
fn unable_to_determine_current_version_errors() {
  let dir = TempDir::new().unwrap();
  let err = version_bump_info(&opts(Some("major")), dir.path()).unwrap_err();
  assert!(
    err
      .to_string()
      .contains("Unable to determine the current version number"),
    "错误对齐上游文案：{err}"
  );
}

#[test]
fn preid_flows_into_pre_release_types() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let opts = BumpInfoOptions {
    release: Some("prepatch"),
    files: &[],
    current_version: None,
    preid: Some("beta"),
  };
  let state = version_bump_info(&opts, &path).unwrap();
  assert_eq!(state.new_version, "1.2.4-beta.1");
}

#[test]
fn preid_defaults_to_beta_like_upstream() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  // 上游 normalizeOptions：preid 缺省为 "beta"
  let state = version_bump_info(&opts(Some("prepatch")), &path).unwrap();
  assert_eq!(state.new_version, "1.2.4-beta.1");
}

#[test]
fn current_version_option_marks_source_as_user() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let opts = BumpInfoOptions {
    release: Some("minor"),
    files: &[],
    current_version: Some("9.9.9"),
    preid: None,
  };
  let state = version_bump_info(&opts, &path).unwrap();
  assert_eq!(
    state.current_version_source, "user",
    "上游 Operation 构造器如此"
  );
}

#[test]
fn state_shape_matches_upstream() {
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir);
  let state = version_bump_info(&opts(Some("major")), &path).unwrap();
  // 上游 state 全字段形状
  assert_eq!(state.commit_message, "");
  assert_eq!(state.tag_name, "");
  assert!(state.updated_files.is_empty());
  assert!(state.skipped_files.is_empty());
}

// ---- prompt 选项文案（纯函数，不含交互） ----

#[test]
fn choices_match_upstream_texts_and_order() {
  let next = next_versions("1.2.3", None, &[]).unwrap();
  let choices = build_choices("1.2.3", &next);
  let values: Vec<&str> = choices.iter().map(|(v, _)| v.as_str()).collect();
  assert_eq!(
    values,
    vec![
      "major",
      "minor",
      "patch",
      "next",
      "conventional",
      "prepatch",
      "preminor",
      "premajor",
      "none",
      "custom",
    ]
  );
  let titles: Vec<&str> = choices.iter().map(|(_, t)| t.as_str()).collect();
  // 上游 padStart(13) 右对齐，custom 为 padStart(17)
  assert_eq!(titles[0], "        major 2.0.0");
  assert_eq!(titles[1], "        minor 1.3.0");
  assert_eq!(titles[2], "        patch 1.2.4");
  assert_eq!(titles[3], "         next 1.2.4");
  assert_eq!(titles[4], " conventional 1.2.4");
  assert_eq!(titles[5], "    pre-patch 1.2.4-0");
  assert_eq!(titles[6], "    pre-minor 1.3.0-0");
  assert_eq!(titles[7], "    pre-major 2.0.0-0");
  assert_eq!(titles[8], "        as-is 1.2.3");
  assert_eq!(titles[9], "       custom ...");
}

#[test]
fn error_display_is_readable() {
  let err = InfoError::UnableToDetermineVersion {
    message: "Unable to determine the current version number. Checked package.json.".to_string(),
  };
  assert!(err.to_string().contains("Unable to determine"));
}

// ---- 版本来源生态化（ADR-0007）：get_current_version 经插件底座链分发 ----

#[test]
fn current_version_scanned_from_cargo_toml_probe() {
  // 纯 Cargo 仓库（无 package.json）：探测表含 cargo.toml，版本来源生态化
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  git(&path, &["init", "-b", "main"]);
  git(&path, &["config", "user.email", "test@example.com"]);
  git(&path, &["config", "user.name", "Test"]);
  fs::write(
    path.join("Cargo.toml"),
    "[package]\nname = \"demo\"\nversion = \"4.5.6\"\n",
  )
  .unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "chore: init"]);
  let state = version_bump_info(&opts(Some("patch")), &path).unwrap();
  assert_eq!(state.current_version, "4.5.6");
  assert_eq!(state.current_version_source, "Cargo.toml");
  assert_eq!(state.new_version, "4.5.7");
}

#[test]
fn current_version_from_workspace_package_literal() {
  // 虚拟 workspace 根：仅 [workspace.package].version 字面量可作版本来源
  let dir = TempDir::new().unwrap();
  let path = dir.path().to_path_buf();
  fs::write(
    path.join("Cargo.toml"),
    "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"7.8.9\"\n",
  )
  .unwrap();
  let state = version_bump_info(&opts(Some("patch")), &path).unwrap();
  assert_eq!(state.current_version, "7.8.9");
  assert_eq!(state.current_version_source, "Cargo.toml");
}

#[test]
fn files_order_determines_version_source() {
  // ADR-0007 已记录后果：按 files 顺序先命中先赢——version_bump 的 normalize
  // 后清单按字典序，Cargo.toml 排在 package.json 前
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir); // package.json 1.2.3
  fs::write(
    path.join("Cargo.toml"),
    "[package]\nname = \"demo\"\nversion = \"9.9.9\"\n",
  )
  .unwrap();
  let opts = BumpInfoOptions {
    release: Some("patch"),
    files: &["Cargo.toml".to_string(), "package.json".to_string()],
    current_version: None,
    preid: None,
  };
  let state = version_bump_info(&opts, &path).unwrap();
  assert_eq!(state.current_version, "9.9.9");
  assert_eq!(state.current_version_source, "Cargo.toml");
}

#[test]
fn probe_list_prefers_node_manifests_in_chain_order() {
  // 空 files → 探测表 = 链序 basename 并集（node 8 项在 cargo.toml 前）
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir); // package.json 1.2.3
  fs::write(
    path.join("Cargo.toml"),
    "[package]\nname = \"demo\"\nversion = \"9.9.9\"\n",
  )
  .unwrap();
  let state = version_bump_info(&opts(Some("patch")), &path).unwrap();
  assert_eq!(state.current_version, "1.2.3");
  assert_eq!(state.current_version_source, "package.json");
}

#[test]
fn non_manifest_files_in_list_are_not_version_sources() {
  // Text 通道文件（README 等）不提供版本；跳过继续探测
  let dir = TempDir::new().unwrap();
  let path = init_repo(&dir); // package.json 1.2.3
  fs::write(path.join("VERSION.txt"), "version 8.8.8\n").unwrap();
  let opts = BumpInfoOptions {
    release: Some("patch"),
    files: &["VERSION.txt".to_string()],
    current_version: None,
    preid: None,
  };
  let state = version_bump_info(&opts, &path).unwrap();
  assert_eq!(state.current_version, "1.2.3");
  assert_eq!(state.current_version_source, "package.json");
}
