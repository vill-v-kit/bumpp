# @vill-v/bumpp

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

本包为 [`@vill-v/bumpp-core`](./napi/bumpp-core)（自研 Rust 版本引擎，语义全量对齐 [bumpp](https://github.com/antfu/bumpp) v11）与 [changelogen](https://github.com/unjs/changelogen) 的组合

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

### scripts：发版钩子命令

`bump.config.json` 可声明三个时序槽位的 shell 命令：

```json
{
  "scripts": {
    "preversion": "cargo fmt --check",
    "version": "pnpm build",
    "postversion": "echo released > stamp.txt"
  }
}
```

时序与上游 npm scripts 位一致：`preversion` 在文件更新前，`version` 在 git 提交前，`postversion` 在 git 完成后。

- **执行方式**：经系统 shell 执行——Unix `sh -c`，Windows `cmd /d /s /c`（与 npm 一致）。`&&`、管道、重定向等 shell 特性均可使用。
- **为什么不是 PowerShell / zsh**：hook 需要在所有协作者机器与 CI 上行为一致。`sh` 是 POSIX 保证的最小公分母（zsh 在多数 Linux 与 CI 镜像中不存在，且与 POSIX sh 有语义差）；`cmd` 是每台 Windows 都自带的解释器（PowerShell 5.1 与 7 语法互不兼容，7 需单独安装）。需要特定 shell 时在命令中显式调用，如 `"preversion": "pwsh -Command \"./scripts/build.ps1\""` 或 `"preversion": "zsh ./scripts/release.zsh"`。
- **失败即中止**：脚本非零退出时发版立即报错中止，不会让失败的构建/校验钩子产出完整发版。
- **跳过**：`"ignoreScripts": true` 跳过全部三个槽位。
- **从上游迁移**：package.json 中的 `preversion` / `version` / `postversion` **不再自动执行**（npm scripts 通道已移除）。需要保留时在 `bump.config.json` 中显式声明，如 `"preversion": "npm run preversion"`。

引擎以预编译二进制分发，当前支持：macOS (arm64)、Linux (x64 / arm64, glibc)、Windows (x64 / arm64)。其他平台安装时会得到列出已支持平台的明确报错。

开发与贡献见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## ❤️ 鸣谢

本项目的版本引擎为 [bumpp](https://github.com/antfu/bumpp) v11 的 Rust 重写（语义全量对齐，Copyright (c) 2022 Anthony Fu、Copyright (c) 2015 James Messinger，[MIT](https://github.com/antfu/bumpp/blob/main/LICENSE)）；`npm/bump/src/changelog.ts` 改写自 [changelogen](https://github.com/unjs/changelogen)（Copyright (c) Pooya Parsa，[MIT](https://github.com/unjs/changelogen/blob/main/LICENSE)）。两个上游的版权行均已保留在根 [LICENSE](./LICENSE) 中。

## License

[MIT](./LICENSE)
