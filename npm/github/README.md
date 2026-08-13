# @vill-v/bumpp-github

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

为 [@vill-v/bumpp](https://npmx.dev/package/@vill-v/bumpp) 的拓展模块

可以在 release 后在 github 上 添加发行说明

## 设置 github access_token

```shell
vbumpp token set github
```

token 加密后以二进制形式安全存储

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

## ❤️ 鸣谢

[bumpp](https://github.com/antfu/bumpp)

[changelogen](https://github.com/unjs/changelogen)

## License

[MIT](https://github.com/vill-v-kit/bumpp/blob/main/LICENSE)
