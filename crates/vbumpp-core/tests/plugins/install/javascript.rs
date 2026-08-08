//! 包管理器检测 parity 矩阵——对齐上游 package-manager-detector 默认行为（ADR-0006）：
//! 同级目录内 lockfile → packageManager 字段，逐级向上爬目录；agent / lockfile 全表。

use std::fs;

use tempfile::TempDir;
use vbumpp_core::plugins::install::javascript::detect_package_manager;

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn package_json_with_pm(pm: &str) -> String {
  format!("{{\n  \"packageManager\": \"{pm}\"\n}}\n")
}

#[test]
fn lockfile_wins_over_package_manager_field_in_same_dir() {
  let dir = TempDir::new().unwrap();
  // 上游默认策略顺序 lockfile → packageManager-field：同级冲突时 lockfile 胜出
  write(&dir, "package.json", &package_json_with_pm("pnpm@10.0.0"));
  write(&dir, "yarn.lock", "");
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "yarn");
}

#[test]
fn package_manager_field_recognizes_all_agents() {
  // 上游 AGENTS 名单（name@version 的 name 部分）：nub / aube / deno 不再误判为 npm
  for (field, agent) in [
    ("npm@10.0.0", "npm"),
    ("yarn@4.1.0", "yarn"),
    ("pnpm@10.0.0", "pnpm"),
    ("bun@1.1.0", "bun"),
    ("deno@1.40.0", "deno"),
    ("nub@0.1.0", "nub"),
    ("aube@0.1.0", "aube"),
  ] {
    let dir = TempDir::new().unwrap();
    write(&dir, "package.json", &package_json_with_pm(field));
    assert_eq!(
      detect_package_manager(dir.path()).unwrap(),
      agent,
      "packageManager 字段 {field} 应识别为 {agent}"
    );
  }
}

#[test]
fn unrecognized_field_falls_through_to_lockfile() {
  let dir = TempDir::new().unwrap();
  // 字段值不在名单内：fall through 到 lockfile 策略（而非旧实现的 _ => "npm" 误判）
  write(&dir, "package.json", &package_json_with_pm("bunpm@1.0.0"));
  write(&dir, "nub.lock", "");
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "nub");
}

#[test]
fn lockfile_table_matches_upstream_locks() {
  // 上游 LOCKS 全表
  for (lock, agent) in [
    ("aube-lock.yaml", "aube"),
    ("aube-workspace.yaml", "aube"),
    ("bun.lock", "bun"),
    ("bun.lockb", "bun"),
    ("deno.lock", "deno"),
    ("nub.lock", "nub"),
    ("pnpm-lock.yaml", "pnpm"),
    ("pnpm-workspace.yaml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("package-lock.json", "npm"),
    ("npm-shrinkwrap.json", "npm"),
  ] {
    let dir = TempDir::new().unwrap();
    write(&dir, lock, "");
    assert_eq!(
      detect_package_manager(dir.path()).unwrap(),
      agent,
      "lockfile {lock} 应识别为 {agent}"
    );
  }
}

#[test]
fn lockfile_order_is_specific_first() {
  let dir = TempDir::new().unwrap();
  // 上游注释"the order here matters, more specific one comes first"
  write(&dir, "nub.lock", "");
  write(&dir, "package-lock.json", "");
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "nub");
}

#[test]
fn crawl_finds_lockfile_in_parent_directory() {
  let dir = TempDir::new().unwrap();
  fs::create_dir_all(dir.path().join("sub/deep")).unwrap();
  write(&dir, "nub.lock", "");
  assert_eq!(
    detect_package_manager(&dir.path().join("sub/deep")).unwrap(),
    "nub"
  );
}

#[test]
fn nearest_directory_level_wins() {
  let dir = TempDir::new().unwrap();
  fs::create_dir_all(dir.path().join("sub")).unwrap();
  write(&dir, "package-lock.json", "");
  write(&dir, "sub/nub.lock", "");
  // 目录为外层循环：最近级先命中
  assert_eq!(
    detect_package_manager(&dir.path().join("sub")).unwrap(),
    "nub"
  );
}

#[test]
fn crawl_finds_package_manager_field_in_parent() {
  let dir = TempDir::new().unwrap();
  fs::create_dir_all(dir.path().join("sub")).unwrap();
  write(&dir, "package.json", &package_json_with_pm("bun@1.1.0"));
  assert_eq!(
    detect_package_manager(&dir.path().join("sub")).unwrap(),
    "bun"
  );
}

#[test]
fn nothing_found_reports_upstream_error_message() {
  let dir = TempDir::new().unwrap();
  let err = detect_package_manager(dir.path()).unwrap_err();
  assert_eq!(
    err.to_string(),
    "Could not detect package manager, failed to run npm install"
  );
}
