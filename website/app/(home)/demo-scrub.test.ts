/**
 * demo-scrub.ts 的行为测试。
 * Seam：进度控制器的纯逻辑边界——滚动进度 → 段落/事件定位（locateAt）、
 * 当前状态 × 目标 → 重放计划（planScrub）、事件区间 → 字节拼接（segmentText）。
 * 数据用真实采集产物 demo-casts.ts（四段演示同一份数据，测试与线上同源）。
 */
import { describe, expect, it } from 'vitest';

import { DEMO_SEGMENTS, type DemoCast } from './demo-casts';
import {
  buildTimeline,
  contentRows,
  locateAt,
  planScrub,
  segmentText,
} from './demo-scrub';

const timeline = buildTimeline(DEMO_SEGMENTS);

describe('locateAt：滚动进度 → 段落与已呈现事件数', () => {
  it('进度 0 时终端停在第一段开头——只有提示符首个事件', () => {
    expect(locateAt(0, timeline)).toEqual({ segmentIndex: 0, eventCount: 1 });
  });

  it('进度 1 时呈现末段全部输出', () => {
    const last = DEMO_SEGMENTS.length - 1;
    expect(locateAt(1, timeline)).toEqual({
      segmentIndex: last,
      eventCount: DEMO_SEGMENTS[last].cast.events.length,
    });
  });

  it('滚过均匀分布的段边界即切换段落，且新段从头部开始', () => {
    const n = DEMO_SEGMENTS.length;
    // 边界前一瞬：上一段；边界上：下一段的事件计数从头计起
    expect(locateAt(1 / n - 1e-9, timeline).segmentIndex).toBe(0);
    const atBoundary = locateAt(1 / n, timeline);
    expect(atBoundary.segmentIndex).toBe(1);
    expect(atBoundary.eventCount).toBeLessThan(
      DEMO_SEGMENTS[1].cast.events.length,
    );
  });

  it('每步窗口前 5% 是段首停留区：只呈现提示符', () => {
    // token-list 段窗口为进度 [0.75, 1]，前 5% 即 [0.75, 0.7625]
    expect(locateAt(0.7625, timeline)).toEqual({
      segmentIndex: 3,
      eventCount: 1,
    });
  });

  it('每步窗口末 15% 是终态停留区：与窗口结束呈现一致', () => {
    const full = DEMO_SEGMENTS[0].cast.events.length;
    // 首段窗口 [0, 0.25]，末 15% 即 [0.2125, 0.25]
    expect(locateAt(0.2125, timeline).eventCount).toBe(full);
    expect(locateAt(0.25 - 1e-9, timeline).eventCount).toBe(full);
  });

  it('窗口中部按事件时间呈现前缀（演算例：token-list 段一半处）', () => {
    // token-list 段时长 1.25s；窗口中点 local=0.45 → 内部进度
    // (0.45-0.05)/0.8=0.5 → t=0.625s；≤0.625s 的事件是提示符
    // 0、0.05、…、0.6 共 13 个（1.2s 起的两行输出未到）
    expect(locateAt(0.8625, timeline)).toEqual({
      segmentIndex: 3,
      eventCount: 13,
    });
  });
});

describe('planScrub：当前状态 × 目标 → 最小重放计划', () => {
  it('首帧（无当前状态）一律重放', () => {
    expect(planScrub(null, { segmentIndex: 0, eventCount: 3 })).toEqual({
      kind: 'replay',
      segmentIndex: 0,
      to: 3,
    });
  });

  it('目标无变化不写终端', () => {
    const at = { segmentIndex: 1, eventCount: 7 };
    expect(planScrub(at, at)).toEqual({ kind: 'none' });
  });

  it('同段向前滚动增量补写新增事件区间', () => {
    expect(
      planScrub(
        { segmentIndex: 1, eventCount: 5 },
        { segmentIndex: 1, eventCount: 9 },
      ),
    ).toEqual({ kind: 'append', segmentIndex: 1, from: 5, to: 9 });
  });

  it('向上回退为重放（重置核心 + 前缀重放）', () => {
    expect(
      planScrub(
        { segmentIndex: 2, eventCount: 10 },
        { segmentIndex: 2, eventCount: 4 },
      ),
    ).toEqual({ kind: 'replay', segmentIndex: 2, to: 4 });
  });

  it('跨段切换为重放到新段目标位置', () => {
    expect(
      planScrub(
        { segmentIndex: 0, eventCount: 46 },
        { segmentIndex: 1, eventCount: 12 },
      ),
    ).toEqual({ kind: 'replay', segmentIndex: 1, to: 12 });
  });
});

describe('segmentText：事件区间 → 输出字节', () => {
  it('拼接 [from, to) 区间的事件数据', () => {
    const cast = DEMO_SEGMENTS[3].cast; // token-list
    // 提示符逐字符事件：0='$'、1=' '、2='v'、3='b'
    expect(segmentText(cast, 0, 4)).toBe('$ vb');
  });

  it('完整前缀等于全段输出（重放的字节来源）', () => {
    const cast = DEMO_SEGMENTS[0].cast;
    const whole = segmentText(cast, 0, cast.events.length);
    expect(whole.startsWith('$ vbumpp --dry-run\r\n')).toBe(true);
    expect(whole).toContain('changelog preview:');
  });
});

describe('contentRows：静态终态卡片的终端行数（信息完整，不留滚动回退区）', () => {
  const fakeCast = (width: number, lines: string[]): DemoCast => ({
    header: { version: 2, width, height: 24, env: {} },
    events: lines.map((line, i) => [i * 0.05, 'o', `${line}\r\n`] as const),
  });

  it('恰好容纳时每行一行，末尾换行留出一行光标位', () => {
    // token-list：提示符 + 两行输出 + 末尾换行的空行
    expect(contentRows(DEMO_SEGMENTS[3].cast)).toBe(4);
    // dry-run：提示符 + 27 行输出 + 空行
    expect(contentRows(DEMO_SEGMENTS[0].cast)).toBe(29);
  });

  it('超过终端列宽的行按折行计行', () => {
    expect(contentRows(fakeCast(10, ['a'.repeat(25)]))).toBe(3 + 1);
  });

  it('CJK 与 emoji 按两列宽计（去 SGR 后）', () => {
    // 6 个 CJK 字符 = 12 列，宽度 10 → 折 2 行；SGR 序列不占宽度
    expect(contentRows(fakeCast(10, ['\x1b[34m修复导出时的编码\x1b[0m']))).toBe(2 + 1);
  });
});
