# @vill-v/bumpp

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

交互选择新版本号 → 生成 CHANGELOG.md → 更新版本文件（monorepo 可整树递归）→ git commit / tag / push → 在代码平台（GitHub / GitLab / Gitee / GitCode）创建 Release，一条命令完成。

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

配置文件为两级多格式：项目级 `.vbumpprc.{json,jsonc,toml}`（`.json` / `.jsonc` 同走 JSONC 解析，支持注释与尾逗号）；全局通用配置放 `~/.vbumpp/config.{json,jsonc,toml}`，合并优先级：overrides > 项目 > 全局 > 内建默认。仅支持纯数据格式，不执行 TS/JS 配置文件。

### scripts：发版钩子命令

配置文件的 `scripts` 键可声明三个时序槽位的 shell 命令（以 `.vbumpprc.json` 为例）：

```json
{
  "scripts": {
    "preversion": "cargo fmt --check",
    "version": "pnpm build",
    "postversion": "echo released > stamp.txt"
  }
}
```

时序：`preversion` 在文件更新前，`version` 在 git 提交前，`postversion` 在 git 完成后。

- **执行方式**：经系统 shell 执行——Unix `sh -c`，Windows `cmd /d /s /c`。`&&`、管道、重定向等 shell 特性均可使用。
- **为什么不是 PowerShell / zsh**：hook 需要在所有协作者机器与 CI 上行为一致。`sh` 是 POSIX 保证的最小公分母（zsh 在多数 Linux 与 CI 镜像中不存在，且与 POSIX sh 有语义差）；`cmd` 是每台 Windows 都自带的解释器（PowerShell 5.1 与 7 语法互不兼容，7 需单独安装）。需要特定 shell 时在命令中显式调用，如 `"preversion": "pwsh -Command \"./scripts/build.ps1\""` 或 `"preversion": "zsh ./scripts/release.zsh"`。
- **失败即中止**：脚本非零退出时发版立即报错中止，不会让失败的构建/校验钩子产出完整发版。
- **跳过**：`"ignoreScripts": true` 跳过全部三个槽位。
- **与同名 npm scripts 的关系**：package.json 中的 `preversion` / `version` / `postversion` 不会被自动执行；需要时在 `scripts` 键中显式调用，如 `"preversion": "npm run preversion"`。

引擎以预编译二进制分发，当前支持：macOS (arm64)、Linux (x64 / arm64，glibc 与 musl)、Windows (x64 / arm64)。不在支持列表的平台在加载原生绑定时会得到明确报错。

完整文档见 <https://vill-v-kit.github.io/bumpp/>；开发与贡献见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## ❤️ 鸣谢

本项目的版本引擎为 [bumpp](https://github.com/antfu/bumpp) v11 的 Rust 重写（语义全量对齐，Copyright (c) 2022 Anthony Fu、Copyright (c) 2015 James Messinger，[MIT](https://github.com/antfu/bumpp/blob/main/LICENSE)）；changelog 生成（[`crates/vbumpp-core/src/changelog/`](./crates/vbumpp-core/src/changelog)）改写自 [changelogen](https://github.com/unjs/changelogen)（Copyright (c) Pooya Parsa，[MIT](https://github.com/unjs/changelogen/blob/main/LICENSE)）。两个上游的版权行均已保留在根 [LICENSE](./LICENSE) 中。

## License

[MIT](./LICENSE)
