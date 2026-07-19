# @vill-v/bumpp-gitcode

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

为 [@vill-v/bumpp](https://npmx.dev/package/@vill-v/bumpp) 的拓展模块

可以在 release 后在 gitcode 上 添加发型说明

## 设置 gitcode access_token

在node项目下，或者用户目录下 创建 `.vbumpprc` 文件

配置文件示例

```npmrc
accesstoken.gitcode=tokenxxxxx
```

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

[MIT](https://gitee.com/vill-v/bump/blob/main/LICENSE)
