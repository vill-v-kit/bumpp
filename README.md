# @vill-v/bumpp

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

本包为 [`@vill-v/bumpp-core`](./npm/bumpp-core)（自研 Rust 版本引擎，语义全量对齐 [bumpp](https://github.com/antfu/bumpp) v11）与 [changelogen](https://github.com/unjs/changelogen) 的组合

解决

- 使用 [bumpp](https://github.com/antfu/bumpp) 无法生成 CHANGELOG.md
- 使用 [changelogen](https://github.com/unjs/changelogen) 对 monorepo项目 无法进行有效的release

## 简单使用

```shell
vbumpp
```

## monorepo项目

```shell
vbumpp -r
```

## 自定义bump文件

```shell
vbumpp package.json packages/*/package.json
```

## 配置说明

版本引擎的配置文件为 `bump.config.json`（**仅支持 JSON**）。如果你曾使用上游 bumpp 的 TS/JS 配置文件（`bump.config.ts` 等），会得到明确的报错与迁移指引；`customVersion` 函数选项因无法以 JSON 表达而不再支持。

引擎以预编译二进制分发，当前支持：macOS (arm64)、Linux (x64 / arm64, glibc)、Windows (x64 / arm64)。其他平台安装时会得到列出已支持平台的明确报错。

开发与贡献见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## ❤️ 鸣谢

本项目的版本引擎为 [bumpp](https://github.com/antfu/bumpp) v11 的 Rust 重写（语义全量对齐，Copyright (c) 2022 Anthony Fu、Copyright (c) 2015 James Messinger，[MIT](https://github.com/antfu/bumpp/blob/main/LICENSE)）；`npm/bump/src/changelog.ts` 改写自 [changelogen](https://github.com/unjs/changelogen)（Copyright (c) Pooya Parsa，[MIT](https://github.com/unjs/changelogen/blob/main/LICENSE)）。两个上游的版权行均已保留在根 [LICENSE](./LICENSE) 中。

## License

[MIT](./LICENSE)
