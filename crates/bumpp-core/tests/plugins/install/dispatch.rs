//! 生态识别与条件触发矩阵（ADR-0008）：更新文件 → 生态集合（含零命中回退），
//! 以及 cargo 适配的真实执行（`cargo check --workspace`）。

use std::fs;

use bumpp_core::plugins::{install::cargo, resolve_ecosystems, Ecosystem};
use tempfile::TempDir;

fn resolve(files: &[&str]) -> Vec<Ecosystem> {
  resolve_ecosystems(&files.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn cargo_toml_maps_to_cargo() {
  assert_eq!(resolve(&["Cargo.toml"]), vec![Ecosystem::Cargo]);
  assert_eq!(resolve(&["crates/a/Cargo.toml"]), vec![Ecosystem::Cargo]);
  assert_eq!(resolve(&["CARGO.TOML"]), vec![Ecosystem::Cargo]);
}

#[test]
fn js_manifest_maps_to_node() {
  assert_eq!(resolve(&["package.json"]), vec![Ecosystem::Node]);
  assert_eq!(resolve(&["package-lock.json"]), vec![Ecosystem::Node]);
  assert_eq!(resolve(&["sub/deno.jsonc"]), vec![Ecosystem::Node]);
}

#[test]
fn both_ecosystems_in_fixed_order() {
  // 固定顺序 Node → Cargo，与 files 清单内顺序无关
  assert_eq!(
    resolve(&["Cargo.toml", "package.json"]),
    vec![Ecosystem::Node, Ecosystem::Cargo]
  );
}

#[test]
fn text_only_updates_fall_back_to_node() {
  // 仅 Text 兜底通道的文件：零生态命中 → 回退 node（上游 --install 语义）
  assert_eq!(
    resolve(&["VERSION.txt", "CHANGELOG.md"]),
    vec![Ecosystem::Node]
  );
}

#[test]
fn cargo_install_runs_cargo_check_on_valid_workspace() {
  let dir = TempDir::new().unwrap();
  fs::write(
    dir.path().join("Cargo.toml"),
    "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
  )
  .unwrap();
  fs::create_dir(dir.path().join("src")).unwrap();
  fs::write(dir.path().join("src/lib.rs"), "").unwrap();
  cargo::install(dir.path()).unwrap();
}

#[test]
fn cargo_install_errors_on_invalid_workspace() {
  let dir = TempDir::new().unwrap();
  fs::write(dir.path().join("Cargo.toml"), "{ not toml").unwrap();
  let err = cargo::install(dir.path()).unwrap_err();
  assert!(!err.to_string().is_empty());
}
