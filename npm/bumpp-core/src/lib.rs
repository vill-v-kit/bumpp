#![deny(clippy::all)]

use napi_derive::napi;

/// Scaffold smoke-test export: proves the Rust → napi → JS link works end to end.
/// Real bumpp APIs land in later tickets (COL-8+).
#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}
