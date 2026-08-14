'use client';

// 首页演示终端的共用件（ADR-0036）：macOS 风格窗口chrome、ReplayCore
// 加载 hook、静态终态卡片与 wasm 失败的纯文本降级。scrollytelling 区与
// 移动端/reduced-motion 降级共用同一份 cast 数据与同一个 wterm 渲染路径
// （静态卡片只是「write 整段」），不养第二套渲染。

import { Terminal, type WTerm } from '@wterm/react';
import '@wterm/react/css';
import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';

import { basePath } from '@/lib/shared';
import type { DemoCast, DemoSegment } from './demo-casts';
import { contentRows, segmentText } from './demo-scrub';
import { ReplayCore } from './replay-core';

/** 加载 wasm 并包出可重放核心；失败（静态导出子路径等）由调用方降级 */
export function useReplayCore(): { core: ReplayCore | null; failed: boolean } {
  const [state, setState] = useState<{
    core: ReplayCore | null;
    failed: boolean;
  }>({ core: null, failed: false });

  useEffect(() => {
    let cancelled = false;
    ReplayCore.load(`${basePath}/wterm.wasm`).then(
      (core) => !cancelled && setState({ core, failed: false }),
      () => !cancelled && setState({ core: null, failed: true }),
    );
    return () => {
      cancelled = true;
    };
  }, []);

  return state;
}

/** macOS 风格窗口chrome：三圆点 + 居中标题（当前演示命令） */
export function TerminalCard({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="overflow-hidden rounded-xl border border-fd-border bg-fd-card shadow-sm">
      <div className="flex items-center border-b border-fd-border px-4 py-2.5">
        <span className="flex gap-1.5" aria-hidden="true">
          <span className="size-3 rounded-full bg-[#ff5f57]" />
          <span className="size-3 rounded-full bg-[#febc2e]" />
          <span className="size-3 rounded-full bg-[#28c840]" />
        </span>
        <span className="flex-1 truncate text-center font-mono text-xs text-fd-muted-foreground">
          {title}
        </span>
        <span className="w-12" aria-hidden="true" />
      </div>
      {children}
    </div>
  );
}

/** wasm 加载/初始化失败的静态降级：同一段内容的纯文本（去 SGR），信息不丢 */
export function CastTextFallback({ cast }: { cast: DemoCast }) {
  const plain = useMemo(
    () => segmentText(cast, 0, cast.events.length).replace(/\x1b\[[0-9;]*m/g, ''),
    [cast],
  );
  return (
    <pre className="overflow-x-auto p-4 font-mono text-sm leading-relaxed">
      {plain}
    </pre>
  );
}

interface DemoTerminalProps {
  core: ReplayCore | null;
  cast: DemoCast;
  /** 行数覆盖：静态终态卡片按内容行数撑开（信息完整、不留滚动回退区） */
  rows?: number;
  onReady: (wt: WTerm) => void;
  onError: () => void;
}

/** 单段 cast 的 wterm 挂载位：核心未就绪时给骨架态 */
export function DemoTerminal({ core, cast, rows, onReady, onError }: DemoTerminalProps) {
  const effectiveRows = rows ?? cast.header.height;
  // wasm 就绪前行网格尚未建，按终态高度预占防跳（行高 17px + 上下
  // padding 24px，与 @wterm/react 自动高度同式）
  return (
    <div
      className="relative"
      style={{ minHeight: effectiveRows * 17 + 24 }}
      aria-busy={!core}
    >
      {core ? (
        <Terminal
          cols={cast.header.width}
          rows={effectiveRows}
          core={core}
          onData={() => {
            // 演示终端不吃键入；给出空回调同时关闭本地回显
          }}
          onReady={onReady}
          onError={onError}
          aria-label="命令演示终端"
          className="demo-terminal overflow-x-auto!"
        />
      ) : (
        <div
          className="demo-terminal absolute inset-0 flex items-center justify-center"
          style={{ background: 'var(--term-bg)' }}
          aria-hidden="true"
        >
          <span
            className="animate-pulse font-mono text-sm"
            style={{ color: 'var(--term-fg)' }}
          >
            终端加载中…
          </span>
        </div>
      )}
    </div>
  );
}

/** 降级形态的静态终态卡片：同一渲染路径，就绪后 write 整段字节 */
export function StaticCastCard({ segment }: { segment: DemoSegment }) {
  const { core, failed } = useReplayCore();
  const [initFailed, setInitFailed] = useState(false);
  const fullText = useMemo(
    () => segmentText(segment.cast, 0, segment.cast.events.length),
    [segment],
  );
  // 行数按内容撑开（含折行与光标位折算）：整段终态全部可见、无滚动回退区
  const rows = useMemo(() => contentRows(segment.cast), [segment]);

  const handleReady = useCallback(
    (wt: WTerm) => {
      // wterm init 会抢焦点，页面场景还给文档流（键盘滚动、tab 序不被终端截走）
      wt.element.blur();
      wt.write(fullText);
    },
    [fullText],
  );
  const handleError = useCallback(() => setInitFailed(true), []);

  return (
    <TerminalCard title={`$ ${segment.command}`}>
      {failed || initFailed ? (
        <CastTextFallback cast={segment.cast} />
      ) : (
        <DemoTerminal
          core={core}
          cast={segment.cast}
          rows={rows}
          onReady={handleReady}
          onError={handleError}
        />
      )}
    </TerminalCard>
  );
}
