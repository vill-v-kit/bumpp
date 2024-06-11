# Changelog


## v1.0.5

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.4...v1.0.5)

### 🏡 框架

- Bump `bumpp@9.4.1` `execa@9.2.0` `ofetch@1.3.4` `rc9@2.1.2` (82e7555)

### ❤️ 贡献者

- Whitekite <1075790909@qq.com>

## v1.0.4

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.3...v1.0.4)

### 🩹 修复

- **bumpp:** 修复因为依赖 `bumpp@9.4.0` 内部 api 的破坏性改动导致的功能异常 (8705fd4)

### ❤️ 贡献者

- Whitekite <1075790909@qq.com>

## v1.0.3

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.2...v1.0.3)

### 🏡 框架

- **build:** 修改项目打包目标为 node18 (15cba0a)

### ❤️ 贡献者

- Whitekite <1075790909@qq.com>

## v1.0.2

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.1...v1.0.2)

## v1.0.1

[compare changes](https://gitee.com/vill-v/bump/compare/v1.0.0...v1.0.1)

### 🩹 Fixes

- 修复尊崇 `changelogen` 配置文件导致的模块自身默认配置被忽略 (fca754d)
- **bumpp:** 修复配置项 `changelog` 允许的配置不准确，ps-虽然尊重 `changelogen` `bumpp` 各自的配置文件，但实际可配置内容,由于本插件使用的性质，并未完全开放，推荐使用插件自身的配置文件 `vbumpp.config.{mc}{tj}s` (746b011)

### ❤️ 贡献者

- Whitekite <1075790909@qq.com>

## v1.0.0

[compare changes](https://gitee.com/vill-v/bump/compare/v0.4.4...v1.0.0)

### 🚀 Enhancements

- **bumpp:** 尊重 `changelog` `bumpp` 各自的配置文件 (d600aab)
- **bumpp-gitee:** 使用 `consola/utils` 替换 `chalk` (22e5787)

### 🩹 Fixes

- **bumpp:** 修复 changelog markdown 标题替换失败 (578ac5d)

### 🏡 Chore

- **bumpp:** Update dep `bumpp^9.3.0` `changelogen^0.5.5` (92d8cf8)
- **dep:** ⚠️  由于以下依赖升级  `ora^8.0.1` `globby^14.0.1` `execa^8.0.1`，修改最低 node 版本 v18 (c832993)
- Update .gitignore (7188e99)

#### 🚨 破坏性改动

- **dep:** ⚠️  由于以下依赖升级  `ora^8.0.1` `globby^14.0.1` `execa^8.0.1`，修改最低 node 版本 v18 (c832993)

### ❤️ 贡献者

- Whitekite <1075790909@qq.com>

## v0.4.4


### 🚀 特性

- 定制 break change，Contributors 生成changelog的文案 (aab98d0)

### ❤️  贡献者

- Whitekite <1075790909@qq.com>

## v0.4.3


### 🏡 框架

- **dep:** Changelogen update to 0.5.4 (1f68bb9)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.4.2


### 🚀 特性

  - **gitee:** 由于 open-api gitee pages build 功能 功能仅限 付费用户，遂删除该功能 (342ba37)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.4.1


### 🚀 特性

  - **gitee:** Change gitee pages build cli command (5f07084)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.4.0


### 🏡 框架

  - ⚠️  Change package to es module (0ec80b3)
  - 补充部分代码注释信息 (28636de)

#### ⚠️  Breaking Changes

  - ⚠️  Change package to es module (0ec80b3)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.3.2


### 🚀 特性

  - **gitee:** Add gitee pages build util (dd8e44b)

### 🏡 框架

  - **gitee:** Update README.md (92dab92)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.3.1


### 🚀 特性

  - **gitee:** Update console log (165071c)

### 📖 文档

  - Update README.md (498c530)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.3.0


### 🚀 特性

  - **gitee:** 增加一个简易的模块 `@vill-v/bumpp-gitee` 提供release 后的gitee操作 (fe4a1ae)

### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)
  - Update CHANGELOG.md (f5b009e)
  - Update CHANGELOG.md (d51f8aa)
  - Update CHANGELOG.md (e94016e)
  - Update CHANGELOG.md (449ab35)
  - Update CHANGELOG.md (552b21c)
  - Update CHANGELOG.md (b8050b4)
  - Release v0.2.4 (132603d)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🚀 特性

  - **gitee:** 增加一个简易的模块 `@vill-v/bumpp-gitee` 提供release 后的gitee操作 (fe4a1ae)

### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)
  - Update CHANGELOG.md (f5b009e)
  - Update CHANGELOG.md (d51f8aa)
  - Update CHANGELOG.md (e94016e)
  - Update CHANGELOG.md (449ab35)
  - Update CHANGELOG.md (552b21c)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)
  - Update CHANGELOG.md (f5b009e)
  - Update CHANGELOG.md (d51f8aa)
  - Update CHANGELOG.md (e94016e)
  - Update CHANGELOG.md (449ab35)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)
  - Update CHANGELOG.md (f5b009e)
  - Update CHANGELOG.md (d51f8aa)
  - Update CHANGELOG.md (e94016e)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)
  - Update CHANGELOG.md (f5b009e)
  - Update CHANGELOG.md (d51f8aa)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)
  - Update CHANGELOG.md (f5b009e)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🏡 框架

  - Update CHANGELOG.md (9c6f730)
  - Update CHANGELOG.md (1176f67)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4


### 🏡 框架

  - Update CHANGELOG.md (9c6f730)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.4

## v0.2.3


### 🚀 特性

  - **deps:** `bumpp@9.1.0` `ora@6.3.0` `changelogen@.5.2` (3b3c5d0)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.2


### 🚀 特性

  - **deps:** `execa@7.1.1` `6.2.0@ora` `c12@1.2.0` (6f3c1c4)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.1


### 🩹 修复

  - 修复 git commit changelog file失败的问题 (2bb753c)
  - 修复 git commit changelog file失败的问题 (ac17568)
  - 修复 git commit changelog file失败的问题 (eb889b8)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.1


### 🩹 修复

  - 修复 git commit changelog file失败的问题 (2bb753c)
  - 修复 git commit changelog file失败的问题 (ac17568)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.1


### 🩹 修复

  - 修复 git commit changelog file失败的问题 (2bb753c)
  - 修复 git commit changelog file失败的问题 (ac17568)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.1


### 🩹 修复

  - 修复 git commit changelog file失败的问题 (2bb753c)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.2.0


### 🚀 特性

  - 升级部分依赖 `changelogen@0.5.1` `execa@7.1.0` (8df5a97)
  - 修改changelog生成默认配置 (9894abd)
  - 修改内部changelog git command  使用`execa@7.1.0` `$` 特性 (932869a)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.1.1


### 🚀 特性

  - 还原 `bumpp@9.0.0` `recursive` 特性的适配 (c4ce200)
  - 增加默认的 `bumpp` 默认文件 `package-lock.json` (b6fd1c6)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.1.0


### 🚀 特性

  - 适配 `bumpp@9.0.0` `recursive` 特性 (4c136cd)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.0.4


### 🏡 框架

  - Update README.md (d2bc879)
  - 修改 changelog 加载动画 (2d8cbfc)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.0.3


### 🚀 特性

  - Changelog 加载动画换行 (5d9551a)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.0.2


### 🚀 特性

  - Changelog 增加加载动画 (e8d50c8)
  - 增加 bumpp.recursive 属性,支持monorepo 项目 (e20fee4)
  - 默认 `bumpp` 当前 `process.cwd()` 下 `package,json` (d5db56e)
  - Bumpp 不再需要二次确认 (020339a)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

## v0.0.1


### 🏡 框架

  - Add README.md (c4de77e)
  - Update README.md (aaec057)

### ❤️  Contributors

- Whitekite <1075790909@qq.com>

