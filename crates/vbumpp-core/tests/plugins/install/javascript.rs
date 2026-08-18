//! 包管理器检测 parity 矩阵——对齐上游 package-manager-detector 默认行为：
//! 同级目录内 lockfile → 顶层 packageManager → devEngines.packageManager，逐级向上
//! 爬目录；agent / lockfile 全表；devEngines 只消费对象形态的 name，无效形态静默回退。

use std::fs;

use tempfile::TempDir;
use vbumpp_core::plugins::install::javascript::detect_package_manager;

fn write(dir: &TempDir, name: &str, content: &str) {
  fs::write(dir.path().join(name), content).unwrap();
}

fn package_json_with_pm(pm: &str) -> String {
  format!("{{\n  \"packageManager\": \"{pm}\"\n}}\n")
}

fn package_json_with_dev_engines(pm: &str) -> String {
  format!("{{\n  \"devEngines\": {{\n    \"packageManager\": {pm}\n  }}\n}}\n")
}

#[test]
fn dev_engines_object_recognizes_all_agents() {
  // devEngines.packageManager 对象形态：只消费 name（version / onFail 不参与调度），
  // 支持名单与顶层 packageManager 字段一致
  for agent in ["npm", "yarn", "pnpm", "bun", "deno", "nub", "aube"] {
    let dir = TempDir::new().unwrap();
    write(
      &dir,
      "package.json",
      &package_json_with_dev_engines(&format!(
        "{{\n      \"name\": \"{agent}\",\n      \"version\": \">=10\",\n      \"onFail\": \"error\"\n    }}"
      )),
    );
    assert_eq!(
      detect_package_manager(dir.path()).unwrap(),
      agent,
      "devEngines.packageManager.name = {agent} 应识别为 {agent}"
    );
  }
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

#[test]
fn lockfile_wins_over_dev_engines_in_same_dir() {
  let dir = TempDir::new().unwrap();
  // 同级检测顺序 lockfile → 顶层 packageManager → devEngines.packageManager
  write(
    &dir,
    "package.json",
    &package_json_with_dev_engines("{ \"name\": \"pnpm\" }"),
  );
  write(&dir, "bun.lock", "");
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "bun");
}

#[test]
fn package_manager_field_wins_over_dev_engines_in_same_dir() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "package.json",
    "{\n  \"packageManager\": \"yarn@4.1.0\",\n  \"devEngines\": {\n    \"packageManager\": { \"name\": \"pnpm\" }\n  }\n}\n",
  );
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "yarn");
}

#[test]
fn invalid_package_manager_field_falls_through_to_dev_engines() {
  let dir = TempDir::new().unwrap();
  // 顶层字段不识别不阻断同级 devEngines 声明
  write(
    &dir,
    "package.json",
    "{\n  \"packageManager\": \"bunpm@1.0.0\",\n  \"devEngines\": {\n    \"packageManager\": { \"name\": \"deno\" }\n  }\n}\n",
  );
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "deno");
}

#[test]
fn dev_engines_in_nearest_dir_wins_over_parent_lockfile() {
  let dir = TempDir::new().unwrap();
  fs::create_dir_all(dir.path().join("sub")).unwrap();
  write(&dir, "yarn.lock", "");
  write(
    &dir,
    "sub/package.json",
    &package_json_with_dev_engines("{ \"name\": \"nub\" }"),
  );
  // 目录为外层循环：子目录 devEngines 声明胜过父目录 lockfile
  assert_eq!(
    detect_package_manager(&dir.path().join("sub")).unwrap(),
    "nub"
  );
}

#[test]
fn dev_engines_invalid_forms_fall_back_to_parent() {
  // 无效声明一律静默回退，不阻断向父目录继续检测（父目录 nub.lock 兜底）
  for (case, pm) in [
    ("字符串形态", "\"pnpm@10.0.0\""),
    ("数组形态", "[{ \"name\": \"pnpm\" }]"),
    ("缺失 name", "{ \"version\": \">=10\" }"),
    ("非字符串 name", "{ \"name\": 10 }"),
    ("未知名称", "{ \"name\": \"bunpm\" }"),
    ("name 含版本号也不拆", "{ \"name\": \"pnpm@10.0.0\" }"),
  ] {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    write(&dir, "nub.lock", "");
    write(&dir, "sub/package.json", &package_json_with_dev_engines(pm));
    assert_eq!(
      detect_package_manager(&dir.path().join("sub")).unwrap(),
      "nub",
      "devEngines.packageManager 为{case}时应回退到父目录 lockfile"
    );
  }
}

#[test]
fn dev_engines_non_object_falls_back() {
  let dir = TempDir::new().unwrap();
  // devEngines 本身非对象同样宽容回退
  write(&dir, "package.json", "{\n  \"devEngines\": \"pnpm\"\n}\n");
  write(&dir, "deno.lock", "");
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "deno");
}

#[test]
fn dev_engines_parses_jsonc_comments_and_trailing_commas() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "package.json",
    "{\n  // JSONC 注释\n  \"devEngines\": {\n    \"packageManager\": {\n      \"name\": \"aube\", // 尾注释\n    },\n  },\n}\n",
  );
  assert_eq!(detect_package_manager(dir.path()).unwrap(), "aube");
}
