# 首页滚动演示:自产 cast 时间线 + wterm 渲染层、dry-run 离线采集、漂移校验

文档网站首页计划从「静态终端卡片 + 优势 grid」升级为滚动驱动的 scrollytelling 区:sticky 终端随滚动依次演示各子命令卖点(bump dry-run / `-r` monorepo / release 补发 / token 管理)。本 ADR 记录四个难逆转的选择:**演示载体**(自产 cast 时间线 + wterm 渲染层,弃视频、外部播放器与自研渲染器)、**采集边界**(全部 dry-run 只读形态、离线确定性)、**产物维护方式**(本地生成提交 + CI 漂移校验)、**seek 语义**(无快照 API 下的重放式 seek)。

## Decisions

### 演示载体:扩展自产采集管线输出 cast 时间线,wterm 作渲染层

- 现有 `website/scripts/capture-home-demo.sh` 管线从「输出最终屏幕文本」扩展为「输出 **asciicast v2 兼容**的帧时间线」:采集侧只做格式转换——把 pty 原始字节流（含 ANSI 颜色序列）按内容行切成 cast 事件（`[time, "o", data]`），**不做任何屏幕仿真**——屏幕状态与 VT 解析全部交给渲染层。时间戳为采集侧合成的固定节奏（提示符逐字符、其余逐行）：真实 dry-run 输出是毫秒级倾泻，真实 wall-clock timing 既不可字节级复现也不适合演示，而字节级可复现是 CI 漂移校验的前提，确定性优先。产物生成 TS 模块（沿用 `demo-terminal.ts` 模式，如 `demo-casts.ts`）提交进 website 代码——不放 `public/` 运行时 fetch，避开静态导出下自拼 basePath 的坑。`terminal-screen.mjs` 的屏幕折叠逻辑就此退役（cast 管线不消费它；静态最终屏文本与 cast 时间线在渲染切换前并存）。
- 渲染层采用 Vercel Labs 的 **wterm**(`@wterm/react`,Apache-2.0):Zig→WASM 终端核心(~12KB)渲染到 DOM,原生文本选择/复制、无障碍、24-bit 色、CJK 宽字符、CSS 变量主题。前端自研代码只剩「cast 时间表 → `write()` 驱动」的进度控制器;asciicast 事件按时间戳喂给 `useTerminal().write`。格式选 asciicast v2 兼容是为了耐久性——渲染层可被替换(asciinema 播放器、未来的更好的库)而数据不用迁。
- **四步演示**(dry-run 计划预览 / `-r` 整树计划 / `release --dry-run` 补发 / `token list`)共用同一套机制;移动端断点以下与 `prefers-reduced-motion` 降级为每步一张静态卡片——同一机制 `write` 整段字节得到终态,不写第二套渲染路径。
- wasm 经 basePath 显式解析(ADR-0020 约束)或走 core 包的内联 wasm;演示区为 client component,静态导出不受影响,首屏给骨架态。

### seek 语义:重放式 seek,不依赖快照 API

- wterm 的 `TerminalCore` 接口没有 reset/snapshot/seek;滚动 scrub 到时间 t 的实现是**新建 core + 同步 `write` 全部 ≤t 的字节**(每段 cast 几 KB~几十 KB,WASM 近原生,重放亚毫秒级,rAF 节流)。接受"seek 是 O(n) 重放"的语义,不向渲染层要快照能力。落地形态(COL-92):`ReplayCore`(WasmBridge 子类)以 `init` 重置代替重建实例,seek 后全行标脏一次保证回退重绘。
- 同段向前滚动允许**增量补写**优化(只写新增事件区间,不重置核心):cast 事件是纯追加输出,增量与从头重放字节等价;跨段与回退仍走完整重放。
- 前端进度控制器因此是唯一自养代码面:进度→重放调度、步骤切换、降级判定,总量很小。

### 采集边界:全部 dry-run / 只读形态,离线确定性

- 四个演示全部用 `--dry-run` / 只读子命令形态采集:走完真实执行的全部只读计算、拦截全部副作用,输出即真实 CLI 文本。dry-run 本身就是卖点,演示它等于展示它。
- fixture 沿用现有确定性设计(固定日期/hash、pinned 输出),并扩展出多包 monorepo 形态与预置加密 keyring 供 `-r` 与 token 演示使用。**不做任何远端 API mock**(GitHub/GitLab/Gitee/GitCode)——mock 会令采集管线复杂度暴涨且引入不确定性,不值。

### 产物维护:本地生成提交 + CI 漂移校验

- 产物(cast 生成的 TS 模块)由本地跑 capture 脚本生成并提交进 git,与现有 `demo-terminal.ts` 模式一致;CI 增加一条校验腿:重建 fixture、重跑采集、与提交产物 diff,不一致即失败。管线的字节级可复现设计让这条校验几乎零成本,而它堵住的正是本方案的命门——CLI 输出变更后演示静默腐烂。落地形态(COL-93):ci.yml 的 demo-drift 腿(macOS 宿主——采集脚本依赖 BSD script(1) 语法)先构建 release CLI 再跑 `scripts/demo-cast-drift.mjs`(根 pnpm 入口 `check:home-demo-cast`)——重跑采集原地重写产物、`git diff --exit-code` 比对提交产物,漂移即红且失败信息带本地再生成命令;该腿不拦 publish(演示漂移是网站内容问题,与 npm/crates 上架解耦)。

## Alternatives considered

- **VHS tape → MP4 + 滚动 scrub `video.currentTime`(被否决)**:CI 可再生(vhs-action)、真实彩色终端输出,但滚动时解码器压力大,WebM 跳关键帧短暂糊屏、MP4 次之;终端外观由 VHS 自己渲染,与站点主题割裂;CI 需要在 runner 上装 ttyd+ffmpeg。自产管线已存在且维护成本已被接受,视频 scrub 是在低端设备上最容易翻车的形态。
- **asciinema 官方播放器嵌 `.cast`(被否决)**:省掉渲染层,`seek()` API 也可控;但播放器 UI、字体、主题几乎不受控,嵌进 fumadocs 首页风格割裂。保留 cast 数据的格式兼容,等于保留未来切换到它的退路。
- **自研 cast→DOM 渲染器(被 wterm 替代)**:原计划手写 ANSI→DOM 渲染与 `terminal-screen.mjs` 屏幕仿真升级。被替代:完整 VT 解析(CJK、24-bit 色、grapheme)是保真度要求最高、最没必要手写的部分,wterm 免费给到且更好;wasm 分层架构(`TerminalCore` 接口可注入自建核心)保证渲染层不被锁死。自研只保留进度控制器。
- **纯手写脚本 + 打字机组件(被否决)**:零采集管线;但输出是手工快照,与真实 CLI 脱钩、静默腐烂,违背本仓「演示必须真实可再生」的既有取向(现有 capture 管线的存在本身就是这个取向的证明)。
- **CI 全量再生产物(被否决)**:产物只由 CI 产出。贡献者门槛过高——本地无法自产演示数据,改一条输出就要等 CI;漂移校验已覆盖「产物与代码一致」的核心诉求。
- **svg-term / termtosvg(被否决)**:产物是 CSS 关键帧动画 SVG,无滚动 seek 钩子,且两个渲染器均无人维护。

## Consequences

- website 新增依赖 `motion`(滚动绑定,首个动画依赖)与 `@wterm/react`(渲染层,0.x 版本——接受其 API 漂移风险,以「渲染层可换、cast 数据不动」兜底);`terminal-screen.mjs` 退役,采集脚本改产 cast 事件流。
- 首页信息架构随之改写:静态终端卡片与优势 grid 被 scrollytelling 区取代(优势文案并入各步骤),hero 与 credits 不动。
- CLI 输出格式变更会令 CI 漂移校验红,演示与文档同步更新成为发版检查单的一环;这是有意的——演示腐烂被显式暴露而非静默发生。
- seek 为重放语义:cast 段落极大(如未来演示长交互会话)时重放成本线性增长,届时需引入快照/分段;当前四段 dry-run 演示远未触及该量级。
- 未来若演示需求超出终端文本范畴(交互演示、真实网络行为),cast 管线不再适用,需另起方案;asciicast v2 兼容格式让届时迁移只换渲染层不换数据。
