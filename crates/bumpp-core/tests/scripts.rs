//! npm scripts 执行——对齐上游 bumpp v11 runNpmScript。

use std::fs;

use bumpp_core::progress::ProgressEvent;
use bumpp_core::scripts::run_npm_script;
use tempfile::TempDir;

fn write(dir: &TempDir, content: &str) {
  fs::write(dir.path().join("package.json"), content).unwrap();
}

#[test]
fn npm_script_runs_and_produces_event() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    r#"{
  "name": "t",
  "version": "1.0.0",
  "scripts": { "version": "node -e \"require('fs').writeFileSync('ran.txt','')\"" }
}
"#,
  );
  let (event, script) = run_npm_script(dir.path(), "version", false)
    .unwrap()
    .expect("脚本存在应执行");
  assert_eq!(event, ProgressEvent::NpmScript);
  assert_eq!(script, "version");
  assert!(dir.path().join("ran.txt").exists(), "脚本应真实执行");
}

#[test]
fn npm_script_missing_is_none() {
  let dir = TempDir::new().unwrap();
  write(&dir, "{\n  \"version\": \"1.0.0\"\n}\n");
  assert!(run_npm_script(dir.path(), "version", false)
    .unwrap()
    .is_none());
}

#[test]
fn npm_script_ignore_scripts_skips() {
  let dir = TempDir::new().unwrap();
  write(
    &dir,
    "{\n  \"version\": \"1.0.0\",\n  \"scripts\": { \"version\": \"node -e \\\"require('fs').writeFileSync('ran.txt','')\\\"\" }\n}\n",
  );
  assert!(run_npm_script(dir.path(), "version", true)
    .unwrap()
    .is_none());
  assert!(!dir.path().join("ran.txt").exists());
}

#[test]
fn npm_script_failure_does_not_propagate() {
  let dir = TempDir::new().unwrap();
  // 上游：npm script 的 x() 未开 throwOnError，非零退出不传播
  write(
    &dir,
    "{\n  \"version\": \"1.0.0\",\n  \"scripts\": { \"version\": \"exit 1\" }\n}\n",
  );
  let result = run_npm_script(dir.path(), "version", false).unwrap();
  assert!(result.is_some(), "脚本执行过即产出事件，即使退出码非零");
}

#[test]
fn npm_script_missing_package_json_errors() {
  let dir = TempDir::new().unwrap();
  // 上游：readJsoncFile 的 ENOENT 会传播
  assert!(run_npm_script(dir.path(), "version", false).is_err());
}

#[test]
fn non_manifest_package_json_is_skipped() {
  let dir = TempDir::new().unwrap();
  // 上游 isManifest 门：version 为数字 → 非 manifest，脚本不执行
  write(
    &dir,
    "{\n  \"version\": 42,\n  \"scripts\": { \"version\": \"exit 0\" }\n}\n",
  );
  assert!(run_npm_script(dir.path(), "version", false)
    .unwrap()
    .is_none());
}

#[test]
fn falsy_script_value_is_skipped() {
  // 上游 Boolean(scripts[script])：空串 / null / false / 0 均不执行
  for value in ["\"\"", "null", "false", "0"] {
    let dir = TempDir::new().unwrap();
    write(
      &dir,
      &format!("{{\n  \"version\": \"1.0.0\",\n  \"scripts\": {{ \"version\": {value} }}\n}}\n"),
    );
    assert!(
      run_npm_script(dir.path(), "version", false)
        .unwrap()
        .is_none(),
      "scripts.version = {value} 不应执行"
    );
  }
}
