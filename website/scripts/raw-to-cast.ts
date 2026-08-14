#!/usr/bin/env node
// pty 原始字节流 → asciicast v2 兼容的演示时间线（demo-casts.ts）。
// capture-home-demo-cast.sh 的捕获后处理步骤——只做格式转换，不做屏幕仿真：
// 屏幕状态与 VT 解析全部归渲染层 wterm（ADR-0036），本脚本保真保留 SGR 原始字节。
//
// 多段合一（COL-91）：一次调用消费多段原始流，合并产出单个 TS 模块——每段一个
// `<ID>_CAST` 常量，外加带稳定 id 的 DEMO_SEGMENTS 有序清单（前端按步骤取用）。
//
// 确定性设计（验收：两次运行产物字节一致）：
//   - 事件按内容行边界（\n）切分，与采集时的 write 分块、到达时序解耦
//   - 时间戳为下方固定节奏常量——真实 dry-run 毫秒级倾泻，真实 wall-clock
//     既不可字节级复现也不适合演示；渲染层按时间戳重放即得该节奏
//   - 演示提示符（`$ ` + 命令逐字符）为演示约定合成事件，非捕获内容
//
// 用法：node raw-to-cast.ts <ts-output> <cols> <rows> <term> <abs-path> [<abs-path>...] -- <id> <command> <raw-input> [<id> <command> <raw-input> ...]
//   `--` 前：产物路径、pty 参数（须与 capture 脚本 stty 钉死的值一致，事件头
//   如实记录）与待洗白的绝对路径（→ ~，任一残留即报错）；`--` 后为段三元组：
//   <id> 为稳定段标识（kebab-case，决定常量名 <ID>_CAST 与 DEMO_SEGMENTS.id），
//   <command> 用于合成提示符，<raw-input> 为该段 pty 原始流。

import { readFileSync, writeFileSync } from 'node:fs';

function usage(): never {
  console.error(
    'usage: node raw-to-cast.ts <ts-output> <cols> <rows> <term> <abs-path> [<abs-path>...] -- <id> <command> <raw-input> [<id> <command> <raw-input> ...]',
  );
  process.exit(2);
}

const argv = process.argv.slice(2);
const sep = argv.indexOf('--');
if (sep === -1) usage();
const [outputPath, cols, rows, term, ...washPaths] = argv.slice(0, sep);
const segmentArgs = argv.slice(sep + 1);
if (
  !outputPath ||
  !cols ||
  !rows ||
  !term ||
  washPaths.length === 0 ||
  segmentArgs.length === 0 ||
  segmentArgs.length % 3 !== 0
) {
  usage();
}

interface Segment {
  id: string;
  command: string;
  inputPath: string;
}

const segments: Segment[] = [];
for (let i = 0; i < segmentArgs.length; i += 3) {
  const [id, command, inputPath] = segmentArgs.slice(i, i + 3);
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(id)) {
    console.error(`error: 段标识须为 kebab-case: ${id}`);
    process.exit(2);
  }
  segments.push({ id, command, inputPath });
}

// 固定节奏（毫秒，整数累加后转秒，避免浮点累加误差破坏字节级可复现）：
// 提示符逐字符 / 回车后首行输出 / 其余逐行
const PROMPT_STEP_MS = 50;
const FIRST_OUTPUT_DELAY_MS = 250;
const LINE_STEP_MS = 50;

type CastEventTuple = [time: number, type: 'o', data: string];

// 单段原始流 → cast 事件流（切分、洗白、节奏全部按内容确定）
function segmentEvents(id: string, command: string, inputPath: string): CastEventTuple[] {
  let text = readFileSync(inputPath, 'utf8');

  // BSD script(1) 在 stdin EOF 时向 pty 回显的 `^D` + 两次退格——捕获机制伪迹，
  // 恒为流首 4 字节；非此形态的机制字节不应出现，交给下方控制字符门禁报错
  if (text.startsWith('^D\x08\x08')) text = text.slice(4);

  // 控制字符门禁：合法内容只允许 CR / LF / ESC（SGR 等）；其余控制字节出现
  // 说明捕获形态变化（交互回显、进度条重绘……），产物不再可信
  for (const [, ch] of text.matchAll(
    /[\x00-\x09\x0b\x0c\x0e-\x18\x1a\x1c-\x1f\x7f]/g,
  )) {
    console.error(
      `error: 段 ${id} 原始流含意外控制字节 0x${ch.charCodeAt(0).toString(16)}`,
    );
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
  const lines: string[] = [];
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

  // 时间戳：整数毫秒累加后转秒；每段从 0 起（段间节奏由前端步骤切换决定）
  let ms = 0;
  const events: CastEventTuple[] = [];
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
  return events;
}

// JSON.stringify 转双引号字面量，换成仓库 TS 风格的单引号
const singleQuoted = (value: string) =>
  `'${JSON.stringify(value).slice(1, -1).replace(/'/g, "\\'")}'`;

// 段标识 → 常量名（dry-run → DRY_RUN_CAST）
const constName = (id: string) => `${id.replace(/-/g, '_').toUpperCase()}_CAST`;

const linesOut = [
  '// 本文件由 website/scripts/capture-home-demo-cast.sh 生成，勿手改。',
  '// 内容：首页滚动演示（ADR-0036）四段——`vbumpp --dry-run` 单包计划预览、',
  '// `vbumpp -r --dry-run` monorepo 整树计划（含 private 包锁步）、',
  '// `vbumpp release 1.1.0 --dry-run --provider github` 平台 Release 补发预览',
  '//（token 来源为预置加密 keyring，假 token）、`vbumpp token list` 加密',
  '// token 清单。全部为临时 fixture 上真实 CLI 的只读输出：pty 原始字节流',
  '//（含 SGR 颜色）按行切成 asciicast v2 兼容事件流，绝对路径已洗白为 ~。',
  '// 时间戳为采集侧固定节奏（真实输出毫秒级倾泻，不可复现也不宜演示）；',
  '// 各段提示符为逐字符合成事件（非捕获内容）。',
  '// 渲染层为 wterm（首页滚动演示区与静态降级卡片共用本数据）；复跑脚本可字节级复现。',
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
];

for (const { id, command, inputPath } of segments) {
  const events = segmentEvents(id, command, inputPath);
  linesOut.push(`export const ${constName(id)}: DemoCast = {`);
  linesOut.push(
    '  header: {',
    '    version: 2,',
    `    width: ${cols},`,
    `    height: ${rows},`,
    `    env: { TERM: '${term}' },`,
    '  },',
    '  events: [',
  );
  for (const [time, type, data] of events) {
    linesOut.push(
      `    [${JSON.stringify(time)}, '${type}', ${singleQuoted(data)}],`,
    );
  }
  linesOut.push('  ],', '};', '');
}

// 稳定段标识：联合类型 + 有序清单（前端按步骤取用的唯一入口）
const ids = segments.map(({ id }) => `'${id}'`).join(' | ');
linesOut.push(`export type DemoSegmentId = ${ids};`, '');
linesOut.push(
  'export interface DemoSegment {',
  '  readonly id: DemoSegmentId;',
  '  readonly command: string;',
  '  readonly cast: DemoCast;',
  '}',
  '',
  'export const DEMO_SEGMENTS: readonly DemoSegment[] = [',
);
for (const { id, command } of segments) {
  linesOut.push(
    `  { id: '${id}', command: ${singleQuoted(command)}, cast: ${constName(id)} },`,
  );
}
linesOut.push('];', '');

writeFileSync(outputPath, `${linesOut.join('\n')}\n`);
