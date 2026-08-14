// 本文件由 website/scripts/capture-home-demo-cast.sh 生成，勿手改。
// 内容：首页滚动演示（ADR-0036）第一段——`vbumpp --dry-run` 在临时
// 单包 fixture 中的真实只读计划预览：pty 原始字节流（含 SGR 颜色）
// 按行切成 asciicast v2 兼容事件流，绝对路径已洗白为 ~。
// 时间戳为采集侧固定节奏（真实输出毫秒级倾泻，不可复现也不宜演示）；
// 提示符 `$ vbumpp --dry-run` 为逐字符合成事件（非捕获内容）。
// 渲染层（wterm）由后续票接入，本文件只承载数据；复跑脚本可字节级复现。

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

