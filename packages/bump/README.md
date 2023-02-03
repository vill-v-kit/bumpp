# @vill-v/bumpp

遵循 semver 规范进行 release 的工具包（暂时只针对个人使用，并没有开放过多的配置项）

本包为
[bumpp](https://github.com/antfu/bumpp)
与
[changelogen](https://github.com/unjs/changelogen)
的组合

解决 
- 使用 [bumpp](https://github.com/antfu/bumpp) 无法生成 CHANGELOG.md
- 使用 [changelogen](https://github.com/unjs/changelogen) 对 monorepo项目 无法进行有效的release

鸣谢
[bumpp](https://github.com/antfu/bumpp)
[changelogen](https://github.com/unjs/changelogen)


## 简单使用

```shell
vbumpp package.json packages/*/package.json
```