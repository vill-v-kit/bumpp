// 本文件由 website/scripts/capture-home-demo-cast.sh 生成，勿手改。
// 内容：首页滚动演示四段——`vbumpp --dry-run` 单包计划预览、
// `vbumpp -r --dry-run` monorepo 整树计划（含 private 包锁步）、
// `vbumpp release 1.1.0 --dry-run --provider github` 平台 Release 补发预览
//（token 来源为预置加密 keyring，假 token）、`vbumpp token list` 加密
// token 清单。全部为临时 fixture 上真实 CLI 的只读输出：pty 原始字节流
//（含 SGR 颜色）按行切成 asciicast v2 兼容事件流，绝对路径已洗白为 ~。
// 时间戳为采集侧固定节奏（真实输出毫秒级倾泻，不可复现也不宜演示）；
// 各段提示符为逐字符合成事件（非捕获内容）。
// 渲染层为 wterm（首页滚动演示区与静态降级卡片共用本数据）；复跑脚本可字节级复现。

export type CastEvent = readonly [time: number, type: 'o', data: string];

export interface DemoCast {
  readonly header: {
    readonly version: 2;
    readonly width: number;
    readonly height: number;
    readonly env: Readonly<Record<string, string>>;
  };
  readonly events: readonly CastEvent[];
}

export const DRY_RUN_CAST: DemoCast = {
  header: {
    version: 2,
    width: 80,
    height: 24,
    env: { TERM: 'xterm-256color' },
  },
  events: [
    [0, 'o', '$'],
    [0.05, 'o', ' '],
    [0.1, 'o', 'v'],
    [0.15, 'o', 'b'],
    [0.2, 'o', 'u'],
    [0.25, 'o', 'm'],
    [0.3, 'o', 'p'],
    [0.35, 'o', 'p'],
    [0.4, 'o', ' '],
    [0.45, 'o', '-'],
    [0.5, 'o', '-'],
    [0.55, 'o', 'd'],
    [0.6, 'o', 'r'],
    [0.65, 'o', 'y'],
    [0.7, 'o', '-'],
    [0.75, 'o', 'r'],
    [0.8, 'o', 'u'],
    [0.85, 'o', 'n'],
    [0.9, 'o', '\r\n'],
    [1.15, 'o', '\u001b[34mℹ\u001b[0m bump plan (dry run — no changes made)\r\n'],
    [1.2, 'o', '\u001b[34mℹ\u001b[0m package.json: update → 1.1.0\r\n'],
    [1.25, 'o', '\u001b[34mℹ\u001b[0m current version: 1.0.0 (source: package.json)\r\n'],
    [1.3, 'o', '\u001b[34mℹ\u001b[0m new version: 1.1.0\r\n'],
    [1.35, 'o', '\u001b[34mℹ\u001b[0m files to write:\r\n'],
    [1.4, 'o', '\u001b[34mℹ\u001b[0m   CHANGELOG.md\r\n'],
    [1.45, 'o', '\u001b[34mℹ\u001b[0m   package.json\r\n'],
    [1.5, 'o', '\u001b[34mℹ\u001b[0m git actions:\r\n'],
    [1.55, 'o', '\u001b[34mℹ\u001b[0m   commit: chore: release v1.1.0\r\n'],
    [1.6, 'o', '\u001b[34mℹ\u001b[0m   tag: v1.1.0\r\n'],
    [1.65, 'o', '\u001b[34mℹ\u001b[0m   git push\r\n'],
    [1.7, 'o', '\u001b[34mℹ\u001b[0m   git push --tags\r\n'],
    [1.75, 'o', '\u001b[34mℹ\u001b[0m changelog preview:\r\n'],
    [1.8, 'o', '## v1.1.0\r\n'],
    [1.85, 'o', '\r\n'],
    [1.9, 'o', '\r\n'],
    [1.95, 'o', '### 🚀 特性\r\n'],
    [2, 'o', '\r\n'],
    [2.05, 'o', '- 新增夜间模式 (3a1a1b6)\r\n'],
    [2.1, 'o', '\r\n'],
    [2.15, 'o', '### 🩹 修复\r\n'],
    [2.2, 'o', '\r\n'],
    [2.25, 'o', '- 修复导出时的编码错误 (ebc6d6a)\r\n'],
    [2.3, 'o', '\r\n'],
    [2.35, 'o', '### ❤️ Contributors\r\n'],
    [2.4, 'o', '\r\n'],
    [2.45, 'o', '- You\r\n'],
  ],
};

export const RECURSIVE_DRY_RUN_CAST: DemoCast = {
  header: {
    version: 2,
    width: 80,
    height: 24,
    env: { TERM: 'xterm-256color' },
  },
  events: [
    [0, 'o', '$'],
    [0.05, 'o', ' '],
    [0.1, 'o', 'v'],
    [0.15, 'o', 'b'],
    [0.2, 'o', 'u'],
    [0.25, 'o', 'm'],
    [0.3, 'o', 'p'],
    [0.35, 'o', 'p'],
    [0.4, 'o', ' '],
    [0.45, 'o', '-'],
    [0.5, 'o', 'r'],
    [0.55, 'o', ' '],
    [0.6, 'o', '-'],
    [0.65, 'o', '-'],
    [0.7, 'o', 'd'],
    [0.75, 'o', 'r'],
    [0.8, 'o', 'y'],
    [0.85, 'o', '-'],
    [0.9, 'o', 'r'],
    [0.95, 'o', 'u'],
    [1, 'o', 'n'],
    [1.05, 'o', '\r\n'],
    [1.3, 'o', '\u001b[34mℹ\u001b[0m bump plan (dry run — no changes made)\r\n'],
    [1.35, 'o', '\u001b[34mℹ\u001b[0m package.json: update → 1.1.0\r\n'],
    [1.4, 'o', '\u001b[34mℹ\u001b[0m packages/core/package.json: update → 1.1.0\r\n'],
    [1.45, 'o', '\u001b[34mℹ\u001b[0m packages/web/package.json: update → 1.1.0\r\n'],
    [1.5, 'o', '\u001b[34mℹ\u001b[0m current version: 1.0.0 (source: package.json)\r\n'],
    [1.55, 'o', '\u001b[34mℹ\u001b[0m new version: 1.1.0\r\n'],
    [1.6, 'o', '\u001b[34mℹ\u001b[0m files to write:\r\n'],
    [1.65, 'o', '\u001b[34mℹ\u001b[0m   CHANGELOG.md\r\n'],
    [1.7, 'o', '\u001b[34mℹ\u001b[0m   package.json\r\n'],
    [1.75, 'o', '\u001b[34mℹ\u001b[0m   packages/core/package.json\r\n'],
    [1.8, 'o', '\u001b[34mℹ\u001b[0m   packages/web/package.json\r\n'],
    [1.85, 'o', '\u001b[34mℹ\u001b[0m git actions:\r\n'],
    [1.9, 'o', '\u001b[34mℹ\u001b[0m   commit: chore: release v1.1.0\r\n'],
    [1.95, 'o', '\u001b[34mℹ\u001b[0m   tag: v1.1.0\r\n'],
    [2, 'o', '\u001b[34mℹ\u001b[0m   git push\r\n'],
    [2.05, 'o', '\u001b[34mℹ\u001b[0m   git push --tags\r\n'],
    [2.1, 'o', '\u001b[34mℹ\u001b[0m changelog preview:\r\n'],
    [2.15, 'o', '## v1.1.0\r\n'],
    [2.2, 'o', '\r\n'],
    [2.25, 'o', '\r\n'],
    [2.3, 'o', '### 🚀 特性\r\n'],
    [2.35, 'o', '\r\n'],
    [2.4, 'o', '- Core 支持增量编译 (e099ee0)\r\n'],
    [2.45, 'o', '\r\n'],
    [2.5, 'o', '### 🩹 修复\r\n'],
    [2.55, 'o', '\r\n'],
    [2.6, 'o', '- Web 修复导出时的编码错误 (99f8982)\r\n'],
    [2.65, 'o', '\r\n'],
    [2.7, 'o', '### ❤️ Contributors\r\n'],
    [2.75, 'o', '\r\n'],
    [2.8, 'o', '- You\r\n'],
  ],
};

export const RELEASE_DRY_RUN_CAST: DemoCast = {
  header: {
    version: 2,
    width: 80,
    height: 24,
    env: { TERM: 'xterm-256color' },
  },
  events: [
    [0, 'o', '$'],
    [0.05, 'o', ' '],
    [0.1, 'o', 'v'],
    [0.15, 'o', 'b'],
    [0.2, 'o', 'u'],
    [0.25, 'o', 'm'],
    [0.3, 'o', 'p'],
    [0.35, 'o', 'p'],
    [0.4, 'o', ' '],
    [0.45, 'o', 'r'],
    [0.5, 'o', 'e'],
    [0.55, 'o', 'l'],
    [0.6, 'o', 'e'],
    [0.65, 'o', 'a'],
    [0.7, 'o', 's'],
    [0.75, 'o', 'e'],
    [0.8, 'o', ' '],
    [0.85, 'o', '1'],
    [0.9, 'o', '.'],
    [0.95, 'o', '1'],
    [1, 'o', '.'],
    [1.05, 'o', '0'],
    [1.1, 'o', ' '],
    [1.15, 'o', '-'],
    [1.2, 'o', '-'],
    [1.25, 'o', 'd'],
    [1.3, 'o', 'r'],
    [1.35, 'o', 'y'],
    [1.4, 'o', '-'],
    [1.45, 'o', 'r'],
    [1.5, 'o', 'u'],
    [1.55, 'o', 'n'],
    [1.6, 'o', ' '],
    [1.65, 'o', '-'],
    [1.7, 'o', '-'],
    [1.75, 'o', 'p'],
    [1.8, 'o', 'r'],
    [1.85, 'o', 'o'],
    [1.9, 'o', 'v'],
    [1.95, 'o', 'i'],
    [2, 'o', 'd'],
    [2.05, 'o', 'e'],
    [2.1, 'o', 'r'],
    [2.15, 'o', ' '],
    [2.2, 'o', 'g'],
    [2.25, 'o', 'i'],
    [2.3, 'o', 't'],
    [2.35, 'o', 'h'],
    [2.4, 'o', 'u'],
    [2.45, 'o', 'b'],
    [2.5, 'o', '\r\n'],
    [2.75, 'o', '\u001b[34mℹ\u001b[0m repo: domain github.com (Github)\r\n'],
    [2.8, 'o', '\u001b[34mℹ\u001b[0m release plan (dry run — no changes made)\r\n'],
    [2.85, 'o', '\u001b[34mℹ\u001b[0m token source: token store\r\n'],
    [2.9, 'o', '\u001b[34mℹ\u001b[0m provider: Github\r\n'],
    [2.95, 'o', '\u001b[34mℹ\u001b[0m host: https://api.github.com\r\n'],
    [3, 'o', '\u001b[34mℹ\u001b[0m repo: you/my-project\r\n'],
    [3.05, 'o', '\u001b[34mℹ\u001b[0m tag_name: v1.1.0\r\n'],
    [3.1, 'o', '\u001b[34mℹ\u001b[0m prerelease: false\r\n'],
    [3.15, 'o', '\u001b[34mℹ\u001b[0m body:\r\n'],
    [3.2, 'o', '## v1.1.0\r\n'],
    [3.25, 'o', '\r\n'],
    [3.3, 'o', '### 🚀 特性\r\n'],
    [3.35, 'o', '\r\n'],
    [3.4, 'o', '- 新增夜间模式\r\n'],
    [3.45, 'o', '\r\n'],
    [3.5, 'o', '### 🩹 修复\r\n'],
    [3.55, 'o', '\r\n'],
    [3.6, 'o', '- 修复导出时的编码错误\r\n'],
    [3.65, 'o', '\u001b[34mℹ\u001b[0m requests:\r\n'],
    [3.7, 'o', '\u001b[34mℹ\u001b[0m   POST https://api.github.com/repos/you/my-project/releases\r\n'],
  ],
};

export const TOKEN_LIST_CAST: DemoCast = {
  header: {
    version: 2,
    width: 80,
    height: 24,
    env: { TERM: 'xterm-256color' },
  },
  events: [
    [0, 'o', '$'],
    [0.05, 'o', ' '],
    [0.1, 'o', 'v'],
    [0.15, 'o', 'b'],
    [0.2, 'o', 'u'],
    [0.25, 'o', 'm'],
    [0.3, 'o', 'p'],
    [0.35, 'o', 'p'],
    [0.4, 'o', ' '],
    [0.45, 'o', 't'],
    [0.5, 'o', 'o'],
    [0.55, 'o', 'k'],
    [0.6, 'o', 'e'],
    [0.65, 'o', 'n'],
    [0.7, 'o', ' '],
    [0.75, 'o', 'l'],
    [0.8, 'o', 'i'],
    [0.85, 'o', 's'],
    [0.9, 'o', 't'],
    [0.95, 'o', '\r\n'],
    [1.2, 'o', '\u001b[34mℹ\u001b[0m github\r\n'],
    [1.25, 'o', '\u001b[34mℹ\u001b[0m gitlab (https://gitlab.com)\r\n'],
  ],
};

export type DemoSegmentId = 'dry-run' | 'recursive-dry-run' | 'release-dry-run' | 'token-list';

export interface DemoSegment {
  readonly id: DemoSegmentId;
  readonly command: string;
  readonly cast: DemoCast;
}

export const DEMO_SEGMENTS: readonly DemoSegment[] = [
  { id: 'dry-run', command: 'vbumpp --dry-run', cast: DRY_RUN_CAST },
  { id: 'recursive-dry-run', command: 'vbumpp -r --dry-run', cast: RECURSIVE_DRY_RUN_CAST },
  { id: 'release-dry-run', command: 'vbumpp release 1.1.0 --dry-run --provider github', cast: RELEASE_DRY_RUN_CAST },
  { id: 'token-list', command: 'vbumpp token list', cast: TOKEN_LIST_CAST },
];

