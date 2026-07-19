# @vill-v/bumpp

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

本包为 [bumpp](https://github.com/antfu/bumpp) 与 [changelogen](https://github.com/unjs/changelogen) 的组合

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

## token 管理

发版需要的各平台 access_token 可通过 CLI 录入，加密后以**二进制形式**安全存储（不会以明文或文本形式落盘）

```shell
# 录入/更新 token（输入时隐藏回显）
vbumpp token set gitee

# 查看已配置的 token 及来源（不显示明文）
vbumpp token list

# 删除 token
vbumpp token remove gitee
```

配置文件（`.vbumpprc`、`vbumpp.config.ts` 等）中的 `accesstoken` 设置会被忽略，上述二进制存储是 token 的唯一文件来源；历史上存放在 `.vbumpprc` 中的 token 请重新 `token set` 迁移

> 加密说明：密钥由设备信息派生（AES-256-GCM），防护目标是避免 token 明文/文本落盘（误提交、备份泄露等场景），并非高安全保险柜；更换设备或用户后需重新 `token set`

## ❤️ 鸣谢

[bumpp](https://github.com/antfu/bumpp)

[changelogen](https://github.com/unjs/changelogen)

## License

[MIT](https://gitee.com/vill-v/bump/blob/main/LICENSE)
