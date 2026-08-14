#!/usr/bin/env node
// pty 原始字节流 → asciicast v2 兼容的演示时间线（demo-casts.ts）。
// capture-home-demo-cast.sh 的捕获后处理步骤——只做格式转换，不做屏幕仿真：
// 屏幕状态与 VT 解析全部归渲染层 wterm（ADR-0036），本脚本保真保留 SGR 原始字节。
//
// 确定性设计（验收：两次运行产物字节一致）：
//   - 事件按内容行边界（\n）切分，与采集时的 write 分块、到达时序解耦
//   - 时间戳为下方固定节奏常量——真实 dry-run 毫秒级倾泻，真实 wall-clock
//     既不可字节级复现也不适合演示；渲染层按时间戳重放即得该节奏
//   - 演示提示符（`$ ` + 命令逐字符）为演示约定合成事件，非捕获内容
//     （沿 demo-terminal.ts「首行提示符非捕获内容」惯例）
//
// 用法：node raw-to-cast.mjs <raw-input> <ts-output> <command> <cols> <rows> <term> <abs-path> [<abs-path>...]
//   <command> 用于合成提示符；<cols>/<rows>/<term> 须与 capture 脚本 stty 钉死
//   的 pty 参数一致（事件头如实记录）；<abs-path> 为待洗白的绝对路径（→ ~），
//   洗白后仍残留任一路径即报错。

import { readFileSync, writeFileSync } from 'node:fs';

const [, , inputPath, outputPath, command, cols, rows, term, ...washPaths] =
  process.argv;
if (
  !inputPath ||
  !outputPath ||
  !command ||
  !cols ||
  !rows ||
  !term ||
  washPaths.length === 0
) {
  console.error(
    'usage: node raw-to-cast.mjs <raw-input> <ts-output> <command> <cols> <rows> <term> <abs-path> [<abs-path>...]',
  );
  process.exit(2);
}

// 固定节奏（毫秒，整数累加后转秒，避免浮点累加误差破坏字节级可复现）：
// 提示符逐字符 / 回车后首行输出 / 其余逐行
const PROMPT_STEP_MS = 50;
const FIRST_OUTPUT_DELAY_MS = 250;
const LINE_STEP_MS = 50;

let text = readFileSync(inputPath, 'utf8');

// BSD script(1) 在 stdin EOF 时向 pty 回显的 `^D` + 两次退格——捕获机制伪迹，
// 恒为流首 4 字节；非此形态的机制字节不应出现，交给下方控制字符门禁报错
if (text.startsWith('^D\x08\x08')) text = text.slice(4);

// 控制字符门禁：合法内容只允许 CR / LF / ESC（SGR 等）；其余控制字节出现
// 说明捕获形态变化（交互回显、进度条重绘……），产物不再可信
for (const [, ch] of text.matchAll(/[\x00-\x09\x0b\x0c\x0e-\x18\x1a\x1c-\x1f\x7f]/g)) {
  console.error(`error: 原始流含意外控制字节 0x${ch.charCodeAt(0).toString(16)}`);
  process.exit(1);
}

// 绝对路径洗白（/private/tmp/... → ~），洗后即验残留
for (const path of washPaths) {
  text = text.split(path).join('~');
  if (text.includes(path)) {
    console.error(`error: 洗白后仍残留绝对路径 ${path}`);
    process.exit(1);
  }
}

// 行切分：每事件一行（保留行尾 \r\n），尾部无换行的残段自成事件
const lines = [];
let start = 0;
while (start < text.length) {
  const nl = text.indexOf('\n', start);
  if (nl === -1) {
    lines.push(text.slice(start));
    break;
  }
  lines.push(text.slice(start, nl + 1));
  start = nl + 1;
}

// 时间戳：整数毫秒累加后转秒
let ms = 0;
const events = [];
for (const ch of `$ ${command}`) {
  events.push([ms / 1000, 'o', ch]);
  ms += PROMPT_STEP_MS;
}
events.push([ms / 1000, 'o', '\r\n']);
ms += FIRST_OUTPUT_DELAY_MS;
for (const line of lines) {
  events.push([ms / 1000, 'o', line]);
  ms += LINE_STEP_MS;
}

const linesOut = [
  '// 本文件由 website/scripts/capture-home-demo-cast.sh 生成，勿手改。',
  '// 内容：首页滚动演示（ADR-0036）第一段——`vbumpp --dry-run` 在临时',
  '// 单包 fixture 中的真实只读计划预览：pty 原始字节流（含 SGR 颜色）',
  '// 按行切成 asciicast v2 兼容事件流，绝对路径已洗白为 ~。',
  '// 时间戳为采集侧固定节奏（真实输出毫秒级倾泻，不可复现也不宜演示）；',
  '// 提示符 `$ vbumpp --dry-run` 为逐字符合成事件（非捕获内容）。',
  '// 渲染层（wterm）由后续票接入，本文件只承载数据；复跑脚本可字节级复现。',
  '',
  'export type CastEvent = readonly [time: number, type: \'o\', data: string];',
  '',
  'export interface DemoCast {',
  '  readonly header: {',
  '    readonly version: 2;',
  '    readonly width: number;',
  '    readonly height: number;',
  '    readonly env: Readonly<Record<string, string>>;',
  '  };',
  '  readonly events: readonly CastEvent[];',
  '}',
  '',
  'export const DRY_RUN_CAST: DemoCast = {',
  '  header: {',
  '    version: 2,',
  `    width: ${cols},`,
  `    height: ${rows},`,
  `    env: { TERM: '${term}' },`,
  '  },',
  '  events: [',
];
// JSON.stringify 转双引号字面量，换成仓库 TS 风格的单引号
const singleQuoted = (value) =>
  `'${JSON.stringify(value).slice(1, -1).replace(/'/g, "\\'")}'`;

for (const [time, type, data] of events) {
  linesOut.push(`    [${JSON.stringify(time)}, '${type}', ${singleQuoted(data)}],`);
}
linesOut.push('  ],', '};', '');

writeFileSync(outputPath, `${linesOut.join('\n')}\n`);
