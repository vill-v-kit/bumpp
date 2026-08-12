# 发版质量门前移:本地 git hook(hk)+ rust 工具链钉版

v6.1.0 发版接连两起事故。其一:COL-76 提交前未跑 `cargo fmt`,而 CI 仅在 `v*` tag 推送时触发(ADR-0021「tag 推送即授权」),fmt/clippy/test 的唯一运行时机就是发版当场——tag CI 首跑即挂,修复要移 tag 重推。其二:musl ×2 新平台包的 OIDC 首发 404(见 ADR-0021 决策④首发仪式),漏做仪式导致 publish-npm 拓扑序中段失败、4/13 包已上架的部分发布态。本 ADR 记录事故复盘后的质量门前移与工具链钉版决策;首发仪式流程本身不变(ADR-0021/0029 已载),其「矩阵扩张 → 必做仪式」的触发绑定缺口另行处理。

## Decisions

- 质量门前移到本地 git hook,runner 用 hk(经 mise.toml 装配):pre-commit 跑 `cargo fmt --all -- --check`(秒级,本次事故的直接防线),pre-push 跑 `cargo clippy --workspace --all-targets`(分钟级,防同类「tag CI 首跑即挂」)。`cargo test` 不进 hook——全量太慢,留给 tag CI。
- hook 装配自动化:根 `package.json` 的 `prepare` 脚本跑 `hk install`(husky 同款模式),任何 `pnpm install` 后自动装好。脚本本体保持裸 `hk install` 不做环境探测——无 hk 环境(未过 mise 的裸 clone)在 install 时硬失败,引导责任由 CONTRIBUTING.md 承载,不把心智负担藏进 npm script。
- 不设服务端门:不引入 main 分支 CI,ADR-0021 的 tag-only 触发面保持不变;tag CI 仍是上架的最终权威门(test + 7 平台 build + test-bindings 全绿才 publish),hook 只是提前量。`--no-verify` / `HK=0` 逃逸口保留给提交者个人判断。
- rust 工具链钉版:mise.toml 写死 `rust = "1.97.1"`。实测 `mise lock` 对 core:rust 后端只锁 spec 字符串(写入 `version = "stable"`、零平台条目),锁不住解析版本——浮动 stable 会让 rustfmt/clippy 规则漂移攒到发版 tag CI 首爆,钉 toml 是唯一手段。升版 = 手动改 mise.toml + `mise lock`;hk(aqua 后端)与 zig 等由 mise.lock checksum 正常锁定。钉版后组件必须显式声明(`components = ["rustfmt", "clippy"]`):CI 上 rustup 默认 profile 不含 rustfmt/clippy,裸钉版本号会让 fmt/clippy 步骤报「component not installed」(v6.1.1 首跑实例);mise 对既有 toolchain 也会补齐声明的组件,本地/CI 双端自愈。
- oxlint/oxfmt 暂不入 hook:仓库现存 11 个 lint error 与 69 个 oxfmt diff,未经全量清理就挂门会阻塞一切提交;待专门清理后再议入门。
- 发版流整体绕 hk:根 `release` 脚本以 `HK=0` 前缀调 vbumpp——hk 的 pre-push 对 annotated tag 对象算 merge-base 必崩(上游缺陷:`pre_push.rs` 把 tag 对象原样当 to_ref,`git.rs` 的 git2 merge_base 拒非 commit、NotFound 兜底分支不覆盖 Invalid;1.54.1/1.55.0 同存,已报上游 [discussions/1196](https://github.com/jdx/hk/discussions/1196)),不兜底则每次发版的 `git push --tags` 都被自己的门拦死(v6.1.1 实例)。发版提交是机械版本号 bump,内容在特性提交推送时已过门;tag CI 仍是最终权威门。

## Consequences

- fmt/clippy 类问题在提交/推送时刻暴露,不再攒到发版夜;残余风险是 hook 依赖本地装配——prepare 覆盖 `pnpm install` 路径,但不覆盖「新机器只 git clone 未装依赖即提交」的裸奔场景,接受。
- 新克隆仓库的开发者只要 `mise install` + `pnpm install` 即获得完整门;无额外手动步骤。
- rust 钉版使本地与 CI 的 rustfmt/clippy 判定逐字节一致;升版是显式动作,可在非发版窗口从容处理格式化漂移。
- 矩阵扩张 → 首发仪式的触发绑定仍是开放缺口:若未来再扩 target 而漏做首发,publish-npm 仍会部分上架。是否在 npm-publish.mjs 加「全新包名」前置检测(发现未上架包名即发布前拦停并输出仪式指引)另行决策。
