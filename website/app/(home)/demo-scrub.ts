/**
 * 首页滚动演示的进度控制器（「seek 语义：重放式 seek」）——
 * 前端唯一自养代码面中的纯逻辑部分：把区块滚动进度定位到
 * 「第几段、该段已呈现到第几个事件」，再对照当前已应用状态给出
 * 最小重放计划（同段向前增量补写 / 其余一律重置重放）。
 * 本模块不碰 wterm 与 DOM，事件区间 → 字节的拼接也在这里，
 * 组件层只负责调度与把字节写进终端。
 */
import type { DemoCast, DemoSegment } from './demo-casts';

/** 已呈现位置：第几段、该段已呈现的事件数（时间 ≤ 当前位置的事件计数） */
export interface ScrubTarget {
  readonly segmentIndex: number;
  readonly eventCount: number;
}

export type ScrubPlan =
  | { readonly kind: 'none' }
  | {
      readonly kind: 'append';
      readonly segmentIndex: number;
      readonly from: number;
      readonly to: number;
    }
  | { readonly kind: 'replay'; readonly segmentIndex: number; readonly to: number };

export interface ScrubTimeline {
  readonly segments: readonly DemoSegment[];
  /** 各段时长（末事件时间戳），定位时把段内窗口映射到事件时间 */
  readonly durations: readonly number[];
}

/** 每步窗口前 5% 停在段首（新命令出现的感知），末 15% 停留终态 */
const LEAD = 0.05;
const HOLD = 0.15;

export function buildTimeline(segments: readonly DemoSegment[]): ScrubTimeline {
  return {
    segments,
    durations: segments.map(
      (segment) => segment.cast.events[segment.cast.events.length - 1][0],
    ),
  };
}

export function locateAt(progress: number, timeline: ScrubTimeline): ScrubTarget {
  const n = timeline.segments.length;
  const p = Math.min(Math.max(progress, 0), 1);
  // 均匀分段：每段占 1/n 进度窗口；p=1 落入末段
  const segmentIndex = Math.min(Math.floor(p * n), n - 1);
  const local = (p - segmentIndex / n) * n;
  const inner = Math.min(Math.max((local - LEAD) / (1 - LEAD - HOLD), 0), 1);
  const t = inner * timeline.durations[segmentIndex];
  const events = timeline.segments[segmentIndex].cast.events;
  let eventCount = 0;
  while (eventCount < events.length && events[eventCount][0] <= t) eventCount++;
  return { segmentIndex, eventCount };
}

/**
 * 对照当前已应用状态给出最小重放计划：
 * 同段向前 → append（事件只增输出，增量补写与从头重放等价）；
 * 跨段、回退、首帧 → replay（重置核心 + 重放全部 ≤t 字节）。
 */
export function planScrub(
  current: ScrubTarget | null,
  target: ScrubTarget,
): ScrubPlan {
  if (
    !current ||
    current.segmentIndex !== target.segmentIndex ||
    target.eventCount < current.eventCount
  ) {
    return { kind: 'replay', segmentIndex: target.segmentIndex, to: target.eventCount };
  }
  if (target.eventCount === current.eventCount) return { kind: 'none' };
  return {
    kind: 'append',
    segmentIndex: target.segmentIndex,
    from: current.eventCount,
    to: target.eventCount,
  };
}

/** 拼接一段 cast 中 [from, to) 事件的输出字节（演示数据只有 'o' 输出事件） */
export function segmentText(cast: DemoCast, from: number, to: number): string {
  let text = '';
  for (let i = from; i < to; i++) text += cast.events[i][2];
  return text;
}

// 宽字符码点区间（wcwidth 常用表）：CJK、Hangul、全角、emoji 等占两列
const WIDE_RANGES: readonly (readonly [number, number])[] = [
  [0x1100, 0x115f],
  [0x2e80, 0xa4cf],
  [0xac00, 0xd7a3],
  [0xf900, 0xfaff],
  [0xfe30, 0xfe6f],
  [0xff00, 0xff60],
  [0xffe0, 0xffe6],
  [0x1f000, 0x1faff],
  [0x20000, 0x3fffd],
];

/** 一行的终端显示宽度（列）。ANSI 序列须先剥离；组合符/VS16 按 1 列计入——
 *  宁可多算一行（多留白）也不漏算（内容会被裁掉） */
function displayWidth(line: string): number {
  let width = 0;
  for (const ch of line) {
    const cp = ch.codePointAt(0) ?? 0;
    width += WIDE_RANGES.some(([lo, hi]) => cp >= lo && cp <= hi) ? 2 : 1;
  }
  return width;
}

/**
 * 静态终态卡片需要的终端行数：整段输出的逻辑行按显示宽度折算折行，
 * 末尾换行符留出一行光标位。行数给足后终端无滚动回退区、内容全可见。
 */
export function contentRows(cast: DemoCast): number {
  const full = segmentText(cast, 0, cast.events.length);
  let rows = 0;
  for (const rawLine of full.split('\n')) {
    const line = rawLine.replace(/\x1b\[[0-9;]*m/g, '').replace(/\r$/, '');
    rows += Math.max(1, Math.ceil(displayWidth(line) / cast.header.width));
  }
  return rows;
}
