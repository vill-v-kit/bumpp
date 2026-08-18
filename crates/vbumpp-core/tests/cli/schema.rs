//! schema 子命令行为矩阵（镜像 src/cli/schema.rs）：
//! stdout 纯 JSON 通路、`--write` 项目级 / 全局落点（home 走 RunEnv 注入，
//! 不碰真实 `~/.vbumpp`）、落点显示路径规范。解析层错误用例
//! 在 parse 子模块。

use std::fs;
use std::path::Path;

use serde_json::{from_str, to_string_pretty, Value};
use tempfile::TempDir;
use vbumpp_core::cli::{run_at, RunEnv};
use vbumpp_core::config::config_schema;
use vbumpp_core::display;

use super::argv;

fn run_schema(items: &[&str], cwd: &Path, home: Option<&Path>) -> (String, String, i32) {
  let argv = argv(items);
  let env = RunEnv {
    store: None,
    cwd: Some(cwd),
    home,
    prompt: None,
    confirm: None,
  };
  let mut out = Vec::new();
  let mut err = Vec::new();
  let code = run_at(&argv, None, &env, &mut out, &mut err);
  (
    String::from_utf8(out).unwrap(),
    String::from_utf8(err).unwrap(),
    code,
  )
}

#[test]
fn schema_stdout_is_pure_json() {
  // 验收锚点：stdout 整体可被 JSON 解析器直接消费，与导出产物逐字同源，
  // 无任何其他打印混入；stderr 空、退出码 0
  let cwd = TempDir::new().unwrap();
  let (out, err, code) = run_schema(&["schema"], cwd.path(), None);
  assert_eq!(code, 0, "退出码");
  assert!(err.is_empty(), "{err}");
  let parsed: Value = from_str(&out).expect("stdout 应为可解析的纯 JSON");
  let expected: Value = from_str(&to_string_pretty(&config_schema()).unwrap()).unwrap();
  assert_eq!(parsed, expected, "stdout 即 config_schema() 导出物");
}

#[test]
fn schema_write_defaults_to_project_level() {
  // `--write` 落点默认项目级 `./vbumpprc.schema.json`（`--project` 缺省），
  // 内容与 stdout 通路同一份；落点按显示路径规范打印（cwd 内打相对）
  let cwd = TempDir::new().unwrap();
  let (out, err, code) = run_schema(&["schema", "--write"], cwd.path(), None);
  assert_eq!(code, 0, "{err}");
  assert!(err.is_empty(), "{err}");
  let target = cwd.path().join("vbumpprc.schema.json");
  let written: Value =
    from_str(&fs::read_to_string(&target).expect("落盘文件应存在")).expect("合法 JSON");
  assert_eq!(written, config_schema(), "落盘内容即导出物");
  assert!(
    out.contains("schema written to vbumpprc.schema.json"),
    "落点显示路径应为 cwd 相对形态：{out}"
  );
}

#[test]
fn schema_write_project_flag_matches_default() {
  // 显式 `--project` 与缺省落点一致
  let cwd = TempDir::new().unwrap();
  let (_out, err, code) = run_schema(&["schema", "--write", "--project"], cwd.path(), None);
  assert_eq!(code, 0, "{err}");
  assert!(
    cwd.path().join("vbumpprc.schema.json").is_file(),
    "--project 落点应与缺省一致"
  );
}

#[test]
fn schema_write_global_uses_vbumpp_home_and_creates_it() {
  // `--global` 落点 `~/.vbumpp/schema.json`——home 走注入（不碰真实家目录）；
  // 家目录不存在时创建（首次使用场景，取嵌套路径一并钉死 create_dir_all）；
  // 落点在 cwd 之外，显示路径打绝对 POSIX 形态
  let cwd = TempDir::new().unwrap();
  let home_parent = TempDir::new().unwrap();
  let home = home_parent.path().join("nested").join(".vbumpp");
  let (out, err, code) = run_schema(&["schema", "--write", "--global"], cwd.path(), Some(&home));
  assert_eq!(code, 0, "{err}");
  assert!(err.is_empty(), "{err}");
  let target = home.join("schema.json");
  let written: Value =
    from_str(&fs::read_to_string(&target).expect("落盘文件应存在")).expect("合法 JSON");
  assert_eq!(written, config_schema(), "落盘内容即导出物");
  assert!(
    out.contains(&format!(
      "schema written to {}",
      display::path(cwd.path(), &target)
    )),
    "落点显示路径应为绝对 POSIX 形态：{out}"
  );
}

#[test]
fn schema_global_without_write_stays_on_stdout() {
  // 落点 flag 只约束 `--write`；未给 `--write` 时仍走 stdout，不落任何盘
  let cwd = TempDir::new().unwrap();
  let home = TempDir::new().unwrap();
  let (out, err, code) = run_schema(&["schema", "--global"], cwd.path(), Some(home.path()));
  assert_eq!(code, 0, "{err}");
  assert!(err.is_empty(), "{err}");
  assert!(from_str::<Value>(&out).is_ok(), "stdout 应为纯 JSON");
  assert!(
    !home.path().join("schema.json").exists(),
    "未给 --write 不得落盘"
  );
  assert!(
    !cwd.path().join("vbumpprc.schema.json").exists(),
    "未给 --write 不得落盘"
  );
}

#[test]
fn schema_target_flags_conflict_and_junk_error() {
  // 命令层报错通路（解析层错误文案在 parse 子模块锚定）：退出码 1、进 stderr
  let cwd = TempDir::new().unwrap();
  let (_out, err, code) = run_schema(&["schema", "--project", "--global"], cwd.path(), None);
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("mutually exclusive"), "{err}");
  let (_out, err, code) = run_schema(&["schema", "--wat"], cwd.path(), None);
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unknown option: --wat"), "{err}");
  let (_out, err, code) = run_schema(&["schema", "foo"], cwd.path(), None);
  assert_eq!(code, 1, "退出码");
  assert!(err.contains("unexpected argument: foo"), "{err}");
}
