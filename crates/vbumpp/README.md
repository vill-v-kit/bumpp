# vbumpp

A semver release tool written in pure Rust: pick a new version, generate a
CHANGELOG, update the version files in your project (or across a monorepo),
then commit, tag, push — and create a Release on your code platform
(GitHub / GitLab / Gitee / GitCode).

This crate ships the native `vbumpp` CLI. The same engine also powers the npm
package [`@vill-v/bumpp`](https://www.npmjs.com/package/@vill-v/bumpp) — the
features are identical, pick whichever distribution fits your environment.

## Install

### Prebuilt binaries via cargo-binstall (recommended, no compilation)

```sh
cargo binstall vbumpp
```

Prebuilt binaries are published on GitHub Releases for 7 targets:

- macOS (Apple Silicon): `aarch64-apple-darwin`
- Linux x64 / arm64 (glibc): `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- Linux x64 / arm64 (musl, fully static — runs on Alpine): `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`
- Windows x64 / arm64: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`

On any platform not on this list (e.g. Intel Mac), `cargo binstall`
automatically falls back to building from source — equivalent to
`cargo install` below.

### Build from source via crates.io

```sh
cargo install vbumpp
```

Re-run the same command to upgrade.

## Usage

Run in a project root (where `package.json` or `Cargo.toml` lives):

```sh
vbumpp    # interactive release
vbumpp -r # monorepo: recursively bump every package in the tree
```

Full documentation: <https://vill-v-kit.github.io/bumpp/>

## License

[MIT](https://github.com/vill-v-kit/bumpp/blob/main/LICENSE)
