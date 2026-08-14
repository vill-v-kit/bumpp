//! Token 存储（ADR-0014）：AES-256-GCM 加密凭证存储，与 JS 时代
//! （npm/bump accesstoken.ts）逐字节兼容——存量 `tokens.bin` 零迁移。
//!
//! 二进制布局: magic "VBTK"(4B) | version(1B) | iv(12B) | authTag(16B) | ciphertext
//! 密钥为存储文件同目录 `key.bin` 的 32 字节随机串（0600），不绑定 hostname 等
//! 易变机器信息。防护级别为「防明文落盘」——持有存储文件、密钥文件与源码者
//! 可解密，非高安全保险柜。

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;

const STORE_MAGIC: &[u8; 4] = b"VBTK";
const STORE_VERSION: u8 = 1;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

#[derive(Debug)]
pub enum TokenError {
  HomeDir { message: String },
  Io { message: String },
  Format { message: String },
  Crypto { message: String },
  Prompt { message: String },
  InvalidHost { message: String },
}

impl fmt::Display for TokenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::HomeDir { message }
      | Self::Io { message }
      | Self::Format { message }
      | Self::Crypto { message }
      | Self::Prompt { message }
      | Self::InvalidHost { message } => f.write_str(message),
    }
  }
}

impl Error for TokenError {}

/// token 存储文件路径：`VBUMPP_TOKEN_STORE` 覆盖（优先级高于 `VBUMPP_HOME`，
/// ADR-0013），默认 `<全局配置目录>/tokens.bin`
pub fn store_path() -> Result<PathBuf, TokenError> {
  if let Some(custom) = env::var_os("VBUMPP_TOKEN_STORE") {
    if !custom.is_empty() {
      return Ok(PathBuf::from(custom));
    }
  }
  Ok(
    crate::home::vbumpp_home()
      .ok_or_else(|| TokenError::HomeDir {
        message:
          "cannot resolve the home directory (neither VBUMPP_HOME nor the system home is available)"
            .into(),
      })?
      .join("tokens.bin"),
  )
}

/// 加密密钥文件路径：与存储文件同目录（JS parity——随 `VBUMPP_TOKEN_STORE` 走）
fn key_path(store: &Path) -> PathBuf {
  store.parent().unwrap_or(Path::new(".")).join("key.bin")
}

/// 读取存储中的全部 token（文件不存在返回空表）
pub fn read_token_store() -> Result<BTreeMap<String, String>, TokenError> {
  read_token_store_at(&store_path()?)
}

pub fn read_token_store_at(store: &Path) -> Result<BTreeMap<String, String>, TokenError> {
  if !store.is_file() {
    return Ok(BTreeMap::new());
  }
  let blob = fs::read(store).map_err(|e| TokenError::Io {
    message: format!(
      "failed to read token store file {}: {e}",
      crate::display::posix(store)
    ),
  })?;
  let key = get_key(store)?;
  let plain = decrypt(&blob, &key)?;
  serde_json::from_slice(&plain).map_err(|e| TokenError::Format {
    message: format!(
      "token store file {} is not valid JSON: {e}",
      crate::display::posix(store)
    ),
  })
}

/// 保存 token（存储文件损坏时从空配置重写——`token set` 始终可用，JS parity）
pub fn save_token(name: &str, token: &str) -> Result<(), TokenError> {
  save_token_at(&store_path()?, name, token)
}

pub fn save_token_at(store: &Path, name: &str, token: &str) -> Result<(), TokenError> {
  let mut tokens = read_token_store_at(store).unwrap_or_default();
  tokens.insert(name.to_owned(), token.to_owned());
  write_token_store(store, &tokens)
}

/// 删除 token（清空后删除存储文件，与 JS 一致不触碰 key.bin）；返回是否实际删除
pub fn remove_token(name: &str) -> Result<bool, TokenError> {
  remove_token_at(&store_path()?, name)
}

pub fn remove_token_at(store: &Path, name: &str) -> Result<bool, TokenError> {
  let mut tokens = read_token_store_at(store)?;
  if tokens.remove(name).is_none() {
    return Ok(false);
  }
  if tokens.is_empty() {
    match fs::remove_file(store) {
      Ok(()) => {}
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
      Err(e) => {
        return Err(TokenError::Io {
          message: format!(
            "failed to delete token store file {}: {e}",
            crate::display::posix(store)
          ),
        })
      }
    }
  } else {
    write_token_store(store, &tokens)?;
  }
  Ok(true)
}

/// 交互录入 token（dialoguer::Password，输入不回显；Esc/Ctrl+C 视为取消返回
/// Ok(None)）。trim 后为空报错——对齐 JS cli 的「token 不能为空」
pub fn prompt_token(name: &str) -> Result<Option<String>, TokenError> {
  match dialoguer::Password::new()
    .with_prompt(format!("Enter the access_token for {name}"))
    .interact()
  {
    Ok(input) => {
      let trimmed = input.trim();
      if trimmed.is_empty() {
        Err(TokenError::Prompt {
          message: "token must not be empty".into(),
        })
      } else {
        Ok(Some(trimmed.to_owned()))
      }
    }
    Err(_) => Ok(None),
  }
}

// ---------------------------------------------------------------------------
// host 作用域键：`provider@host`（如 `gitlab@https://gitlab-a.com`）。
// provider 级旧键零迁移保留，两级键共存。写入（token set）与读取（后续
// release 解析链——host 作用域键优先、provider 级键回落，尚未落地）共用
// 同一规范化函数——两侧归一一致才能保证相撞到同一键。
// ---------------------------------------------------------------------------

/// host 规范化为 base URL：无 scheme 自动补 `https://`（显式 `http://` 原样
/// 保留，覆盖内网纯 HTTP 实例；其余 scheme 拒绝）；scheme 与 authority
/// （host+port）小写，路径大小写敏感不动；去尾斜杠；端口与路径保留
/// （兼容 GitLab relative-url-root 部署）
pub fn normalize_host(raw: &str) -> Result<String, TokenError> {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return Err(TokenError::InvalidHost {
      message: "host must not be empty".into(),
    });
  }
  let with_scheme = if trimmed.contains("://") {
    trimmed.to_owned()
  } else {
    format!("https://{trimmed}")
  };
  let (scheme, rest) = with_scheme.split_once("://").expect("contains checked");
  let scheme = scheme.to_ascii_lowercase();
  if scheme != "http" && scheme != "https" {
    return Err(TokenError::InvalidHost {
      message: format!("unsupported host scheme: {scheme} (expected http or https)"),
    });
  }
  let (authority, path) = match rest.find('/') {
    Some(i) => (&rest[..i], &rest[i..]),
    None => (rest, ""),
  };
  if authority.is_empty() {
    return Err(TokenError::InvalidHost {
      message: format!("invalid host: {raw} (missing host name)"),
    });
  }
  let authority = authority.to_ascii_lowercase();
  // 尾斜杠全去：根路径 `/` 与 `/gitlab/` 分别归一为 `` 与 `/gitlab`
  let path = path.trim_end_matches('/');
  Ok(format!("{scheme}://{authority}{path}"))
}

/// host 作用域复合键：`provider@<规范化 host>`。原始 host 入参经
/// `normalize_host` 归一后拼键——调用方无需自行规范化
pub fn host_scoped_key(provider: &str, raw_host: &str) -> Result<String, TokenError> {
  Ok(format!("{provider}@{}", normalize_host(raw_host)?))
}

/// 键拆解（列表友好显示用）：host 作用域键返回 `(provider, Some(host))`，
/// provider 级键返回 `(key, None)`。host 部分必须带 scheme——JS 时代任意
/// name 均可录入，无 scheme 的 `@` 键按不透明 name 处理（零迁移）
pub fn split_key(key: &str) -> (&str, Option<&str>) {
  match key.split_once('@') {
    Some((provider, host)) if host.contains("://") => (provider, Some(host)),
    _ => (key, None),
  }
}

/// 获取加密密钥；不存在时随机生成并落盘（0600）
fn get_key(store: &Path) -> Result<[u8; KEY_LEN], TokenError> {
  let key_file = key_path(store);
  if key_file.is_file() {
    let raw = fs::read(&key_file).map_err(|e| TokenError::Io {
      message: format!(
        "failed to read key file {}: {e}",
        crate::display::posix(&key_file)
      ),
    })?;
    return <[u8; KEY_LEN]>::try_from(raw.as_slice()).map_err(|_| TokenError::Format {
      message: format!(
        "key file {} is not 32 bytes long",
        crate::display::posix(&key_file)
      ),
    });
  }
  let mut key = [0u8; KEY_LEN];
  rand::rng().fill_bytes(&mut key);
  ensure_dir(key_file.parent().unwrap_or(Path::new(".")))?;
  write_private(&key_file, &key)?;
  Ok(key)
}

/// 加密为二进制存储格式（aes-gcm crate 输出 ct||tag，落盘布局为 iv|tag|ct——
/// 与 Node `cipher.getAuthTag()` 的顺序一致，逐字节兼容的关键位）
fn encrypt(plain: &[u8], key: &[u8; KEY_LEN]) -> Vec<u8> {
  let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
  let mut iv = [0u8; IV_LEN];
  rand::rng().fill_bytes(&mut iv);
  let sealed = cipher
    .encrypt(Nonce::from_slice(&iv), plain)
    .expect("AES-GCM encryption cannot fail");
  let (ct, tag) = sealed.split_at(sealed.len() - TAG_LEN);
  let mut out = Vec::with_capacity(STORE_MAGIC.len() + 1 + IV_LEN + TAG_LEN + ct.len());
  out.extend_from_slice(STORE_MAGIC);
  out.push(STORE_VERSION);
  out.extend_from_slice(&iv);
  out.extend_from_slice(tag);
  out.extend_from_slice(ct);
  out
}

/// 解密二进制存储格式
fn decrypt(blob: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>, TokenError> {
  let header_len = STORE_MAGIC.len() + 1 + IV_LEN + TAG_LEN;
  if blob.len() < header_len || &blob[..STORE_MAGIC.len()] != STORE_MAGIC {
    return Err(TokenError::Format {
      message: "token store file has an invalid format".into(),
    });
  }
  let version = blob[STORE_MAGIC.len()];
  if version != STORE_VERSION {
    return Err(TokenError::Format {
      message: format!("unsupported token store version: {version}"),
    });
  }
  let iv_start = STORE_MAGIC.len() + 1;
  let iv = &blob[iv_start..iv_start + IV_LEN];
  let tag = &blob[iv_start + IV_LEN..header_len];
  let ct = &blob[header_len..];
  let mut sealed = Vec::with_capacity(ct.len() + TAG_LEN);
  sealed.extend_from_slice(ct);
  sealed.extend_from_slice(tag);
  Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key))
    .decrypt(Nonce::from_slice(iv), sealed.as_ref())
    .map_err(|_| TokenError::Crypto {
      message: "failed to decrypt token store file (key mismatch or corrupted file)".into(),
    })
}

/// 全部 token 整体加密写入（JSON 紧凑序列化，对齐 JS `JSON.stringify`）
fn write_token_store(store: &Path, tokens: &BTreeMap<String, String>) -> Result<(), TokenError> {
  let key = get_key(store)?;
  let json = serde_json::to_vec(tokens).expect("BTreeMap serialization cannot fail");
  if let Some(dir) = store.parent() {
    ensure_dir(dir)?;
  }
  write_private(store, &encrypt(&json, &key))
}

/// 目录确保（unix 0700——私有目录，每次写入顺带自愈权限）
fn ensure_dir(dir: &Path) -> Result<(), TokenError> {
  fs::create_dir_all(dir).map_err(|e| TokenError::Io {
    message: format!(
      "failed to create directory {}: {e}",
      crate::display::posix(dir)
    ),
  })?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| TokenError::Io {
      message: format!(
        "failed to set permissions on directory {}: {e}",
        crate::display::posix(dir)
      ),
    })?;
  }
  Ok(())
}

/// 私有文件写入（unix 0600）
fn write_private(path: &Path, data: &[u8]) -> Result<(), TokenError> {
  fs::write(path, data).map_err(|e| TokenError::Io {
    message: format!("failed to write {}: {e}", crate::display::posix(path)),
  })?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| TokenError::Io {
      message: format!(
        "failed to set permissions on {}: {e}",
        crate::display::posix(path)
      ),
    })?;
  }
  Ok(())
}
