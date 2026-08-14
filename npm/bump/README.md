# @vill-v/bumpp

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

交互选择新版本号 → 生成 CHANGELOG.md → 更新版本文件（monorepo 可整树递归）→ git commit / tag / push → 在代码平台（GitHub / GitLab / Gitee / GitCode）创建 Release，一条命令完成。

引擎以预编译二进制分发，当前支持：macOS (arm64)、Linux (x64 / arm64，glibc 与 musl)、Windows (x64 / arm64)。完整文档见 <https://vill-v-kit.github.io/bumpp/>

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

## 配置文件

项目级 `.vbumpprc.{json,jsonc,toml}`（`.json` 与 `.jsonc` 均支持注释与尾逗号）；全局通用配置放 `~/.vbumpp/config.{json,jsonc,toml}`，合并优先级：overrides > 项目 > 全局 > 内建默认

## token 管理

发版需要的各平台 access_token 可通过 CLI 录入，加密后以**二进制形式**安全存储（不会以明文或文本形式落盘）

```shell
# 录入/更新 token（输入时隐藏回显）
vbumpp token set gitee

# 自建 GitLab 实例按实例录入（与项目配置 gitlab.host 对应）
vbumpp token set gitlab --host https://gitlab.example.com

# 查看已配置的 token（不显示明文；host 条目显示为 gitlab (https://...)）
vbumpp token list
vbumpp token list --host https://gitlab.example.com

# 删除 token（默认先列清单再确认；--yes 跳过确认，--dry-run 只看清单）
vbumpp token remove gitee
vbumpp token remove gitlab --host https://gitlab.example.com
vbumpp token remove gitlab --all   # provider 级键 + 全部 host 条目
```

token 加密存储于 `~/.vbumpp/tokens.bin`（可用 `VBUMPP_TOKEN_STORE` 覆盖路径），是 token 的唯一文件来源

> 加密说明：密钥为同目录 `key.bin` 中随机生成的 32 字节串（AES-256-GCM），防护目标是避免 token 明文/文本落盘（误提交、备份泄露等场景），并非高安全保险柜；删除 `key.bin` 后已存储的 token 无法解密，需重新 `token set`

## ❤️ 鸣谢

[bumpp](https://github.com/antfu/bumpp)

[changelogen](https://github.com/unjs/changelogen)

## License

[MIT](https://github.com/vill-v-kit/bumpp/blob/main/LICENSE)
