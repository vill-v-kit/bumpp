# @vill-v/bumpp-gitlab

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

为 [@vill-v/bumpp](https://npmx.dev/package/@vill-v/bumpp) 的拓展模块

可以在 release 后在 gitlab/ gitlab CE EE 上 添加发行说明

## 设置 gitlab access_token

```shell
vbumpp token set gitlab
```

token 加密后以二进制形式安全存储

如果使用的是私有部署的 gitlab，可在 `.vbumpprc` 中设置访问的基础路径（默认：https://gitlab.com）

```npmrc
gitlab.host=http://192.168.31.31
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
