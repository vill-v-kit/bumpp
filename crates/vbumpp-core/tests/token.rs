//! Token 存储：Node 生成样本的 golden test + 行为矩阵。
//! 格式逐字节兼容 JS 时代 accesstoken.ts（magic | version | iv | authTag | ct）。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use vbumpp_core::token::{
  host_scoped_key, normalize_host, read_token_store_at, remove_token_at, save_token_at, split_key,
};

fn fixture_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/token")
}

fn store_in(dir: &TempDir) -> PathBuf {
  dir.path().join("tokens.bin")
}

#[test]
fn golden_node_generated_store_decrypts() {
  let tokens = read_token_store_at(&fixture_dir().join("tokens.bin")).unwrap();
  let expected: BTreeMap<String, String> =
    serde_json::from_str(&fs::read_to_string(fixture_dir().join("expected.json")).unwrap())
      .unwrap();
  assert_eq!(tokens, expected, "Node 版加密产物必须可被 Rust 解密");
}

#[test]
fn missing_store_reads_empty() {
  let dir = TempDir::new().unwrap();
  assert!(read_token_store_at(&store_in(&dir)).unwrap().is_empty());
}

#[test]
fn save_then_read_roundtrip() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "token-a").unwrap();
  save_token_at(&store, "github", "token-b").unwrap();
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens["gitee"], "token-a");
  assert_eq!(tokens["github"], "token-b");
  assert!(
    store.with_file_name("key.bin").is_file(),
    "key.bin 应随首写生成"
  );
}

#[test]
fn save_overwrites_same_name() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "old").unwrap();
  save_token_at(&store, "gitee", "new").unwrap();
  assert_eq!(read_token_store_at(&store).unwrap()["gitee"], "new");
}

#[cfg(unix)]
#[test]
fn store_and_key_files_are_private() {
  use std::os::unix::fs::PermissionsExt;
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "x").unwrap();
  let store_mode = fs::metadata(&store).unwrap().permissions().mode() & 0o777;
  let key_mode = fs::metadata(store.with_file_name("key.bin"))
    .unwrap()
    .permissions()
    .mode()
    & 0o777;
  assert_eq!(store_mode, 0o600, "存储文件权限");
  assert_eq!(key_mode, 0o600, "密钥文件权限");
}

#[test]
fn corrupt_store_read_errors_and_save_self_heals() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  fs::write(&store, b"not a token store").unwrap();
  assert!(read_token_store_at(&store).is_err(), "损坏文件读取应报错");
  save_token_at(&store, "gitee", "healed").unwrap();
  assert_eq!(
    read_token_store_at(&store).unwrap()["gitee"],
    "healed",
    "损坏后 save 从空重写（JS parity）"
  );
}

#[test]
fn wrong_key_fails_decrypt() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "secret").unwrap();
  // 换掉密钥（模拟「设备或用户已变更」场景）——GCM tag 校验必须失败
  fs::write(store.with_file_name("key.bin"), [0xAAu8; 32]).unwrap();
  let err = read_token_store_at(&store).unwrap_err();
  assert!(err.to_string().contains("failed to decrypt"), "{err}");
}

#[test]
fn bad_magic_and_unknown_version_error() {
  let dir = TempDir::new().unwrap();
  let bad_magic = dir.path().join("bad-magic.bin");
  fs::write(&bad_magic, b"XXXX\x01rest").unwrap();
  assert!(read_token_store_at(&bad_magic)
    .unwrap_err()
    .to_string()
    .contains("invalid format"));

  // 合法 magic + 未知版本号：翻版 fixture 字节的 version 位
  let mut blob = fs::read(fixture_dir().join("tokens.bin")).unwrap();
  blob[4] = 99;
  let bad_version = dir.path().join("bad-version.bin");
  fs::write(&bad_version, blob).unwrap();
  // key.bin 需与 store 同目录——拷 fixture 的 key
  fs::copy(fixture_dir().join("key.bin"), dir.path().join("key.bin")).unwrap();
  assert!(read_token_store_at(&bad_version)
    .unwrap_err()
    .to_string()
    .contains("unsupported token store version: 99"));
}

#[test]
fn remove_returns_false_when_absent() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "x").unwrap();
  assert!(!remove_token_at(&store, "github").unwrap());
  assert_eq!(
    read_token_store_at(&store).unwrap().len(),
    1,
    "误删不存在键不动存储"
  );
}

#[test]
fn remove_keeps_store_while_tokens_remain() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "a").unwrap();
  save_token_at(&store, "github", "b").unwrap();
  assert!(remove_token_at(&store, "gitee").unwrap());
  assert!(store.is_file(), "尚有 token 时保留存储文件");
  assert_eq!(read_token_store_at(&store).unwrap()["github"], "b");
}

#[test]
fn remove_deletes_store_file_when_emptied() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitee", "only").unwrap();
  assert!(remove_token_at(&store, "gitee").unwrap());
  assert!(!store.exists(), "清空后删除存储文件（JS parity）");
  assert!(
    store.with_file_name("key.bin").is_file(),
    "key.bin 不随清空删除（JS parity）"
  );
  assert!(read_token_store_at(&store).unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// host 作用域键：normalize_host / host_scoped_key / split_key
// ---------------------------------------------------------------------------

#[test]
fn normalize_host_fills_https_scheme() {
  assert_eq!(
    normalize_host("gitlab-a.com").unwrap(),
    "https://gitlab-a.com"
  );
}

#[test]
fn normalize_host_collapses_equivalent_forms() {
  // 规范化相撞：三种写法归一到同一 host（写入与读取共用同一函数）
  let expected = "https://gitlab-a.com";
  assert_eq!(normalize_host("gitlab-a.com").unwrap(), expected);
  assert_eq!(normalize_host("https://gitlab-a.com/").unwrap(), expected);
  assert_eq!(normalize_host("HTTPS://GitLab-A.com").unwrap(), expected);
}

#[test]
fn normalize_host_preserves_explicit_http() {
  // 显式 http:// 原样保留（内网纯 HTTP 实例）
  assert_eq!(
    normalize_host("http://gitlab.internal").unwrap(),
    "http://gitlab.internal"
  );
}

#[test]
fn normalize_host_preserves_port_and_path() {
  // 端口与路径保留（relative-url-root 部署）；路径大小写敏感不动
  assert_eq!(
    normalize_host("gitlab-a.com:8443/GitLab").unwrap(),
    "https://gitlab-a.com:8443/GitLab"
  );
  assert_eq!(
    normalize_host("https://gitlab-a.com:8443/gitlab/").unwrap(),
    "https://gitlab-a.com:8443/gitlab"
  );
}

#[test]
fn normalize_host_rejects_invalid_input() {
  assert!(normalize_host("").is_err(), "空 host 报错");
  assert!(normalize_host("   ").is_err(), "纯空白 host 报错");
  assert!(normalize_host("https://").is_err(), "空 authority 报错");
  assert!(
    normalize_host("ftp://gitlab-a.com").is_err(),
    "非 http(s) scheme 报错"
  );
}

#[test]
fn host_scoped_key_composes_provider_at_normalized_host() {
  assert_eq!(
    host_scoped_key("gitlab", "gitlab-a.com").unwrap(),
    "gitlab@https://gitlab-a.com"
  );
  assert_eq!(
    host_scoped_key("gitlab", "HTTPS://GitLab-A.com/").unwrap(),
    "gitlab@https://gitlab-a.com",
    "复合键经同一规范化函数，与 set/list 两侧相撞一致"
  );
}

#[test]
fn split_key_round_trips_scoped_and_plain_keys() {
  assert_eq!(
    split_key("gitlab@https://gitlab-a.com"),
    ("gitlab", Some("https://gitlab-a.com"))
  );
  assert_eq!(split_key("github"), ("github", None));
  // JS 时代任意 name 均可录入——无 scheme 的 @ 键按不透明 name 处理（零迁移）
  assert_eq!(split_key("foo@bar"), ("foo@bar", None));
}

#[test]
fn scoped_key_coexists_with_provider_key() {
  let dir = TempDir::new().unwrap();
  let store = store_in(&dir);
  save_token_at(&store, "gitlab", "plain-token").unwrap();
  let scoped = host_scoped_key("gitlab", "https://gitlab-a.com").unwrap();
  save_token_at(&store, &scoped, "scoped-token").unwrap();
  let tokens = read_token_store_at(&store).unwrap();
  assert_eq!(tokens["gitlab"], "plain-token", "provider 级旧键原样保留");
  assert_eq!(tokens[&scoped], "scoped-token");
  assert_eq!(tokens.len(), 2);
}
