'use client';

// 首页滚动演示的播放基座（COL-90）：把 cast 时间线按事件时间戳依次
// write 进 wterm 终端，进视口自动播放一遍。wasm 经 basePath 显式解析
// （静态导出子路径部署，不靠包内 base64 内联的根路径默认值）；滚动
// scrub、多段切换等交互留给后续票，本组件只验证「cast 数据 → wterm
// 渲染」链路（颜色、CJK、宽字符）。

import { Terminal, useTerminal, type WTerm } from '@wterm/react';
import '@wterm/react/css';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { basePath } from '@/lib/shared';
import type { DemoCast } from './demo-casts';

export function DemoCastPlayer({ cast }: { cast: DemoCast }) {
  const { ref, write } = useTerminal();
  const wrapRef = useRef<HTMLDivElement>(null);
  // 播放一旦启动不重来（单段自动播放一遍）；进视口与 wasm 就绪两个
  // 触发源先后不定，都汇到 startPlayback，由 startedRef 收敛
  const startedRef = useRef(false);
  const readyRef = useRef(false);
  const inViewRef = useRef(false);
  const reducedMotionRef = useRef(false);
  const timersRef = useRef<number[]>([]);
  const [wasmReady, setWasmReady] = useState(false);
  const [failed, setFailed] = useState(false);

  // 只消费 asciicast v2 的输出事件（'i'/'m'/'r' 等类型属交互录制，演示无）
  const outputEvents = useMemo(
    () => cast.events.filter((event) => event[1] === 'o'),
    [cast],
  );
  const fullText = useMemo(
    () => outputEvents.map((event) => event[2]).join(''),
    [outputEvents],
  );

  const startPlayback = useCallback(() => {
    // wasm 未就绪时 write 是静默 no-op：必须等 init 完成后才可启动，
    // 否则先视口后就绪的时序会把事件写丢、再被终态补写重复
    if (startedRef.current || !readyRef.current) return;
    startedRef.current = true;
    // 减少动态效果：不逐事件播放，直接呈现终态
    if (reducedMotionRef.current) {
      write(fullText);
      return;
    }
    timersRef.current = outputEvents.map((event) =>
      window.setTimeout(() => write(event[2]), event[0] * 1000),
    );
  }, [fullText, outputEvents, write]);

  const handleReady = useCallback(
    (wt: WTerm) => {
      // wterm init 会抢焦点，页面场景还给文档流（键盘滚动、tab 序不被终端截走）
      wt.element.blur();
      readyRef.current = true;
      setWasmReady(true);
      if (startedRef.current) {
        // dev 严格模式重挂载会销毁重建终端：已播过的直接补写终态
        write(fullText);
        return;
      }
      if (reducedMotionRef.current || inViewRef.current) startPlayback();
    },
    [fullText, startPlayback, write],
  );

  const handleError = useCallback(() => {
    setFailed(true);
  }, []);

  useEffect(() => {
    const wrapper = wrapRef.current;
    if (!wrapper) return;
    reducedMotionRef.current = window.matchMedia(
      '(prefers-reduced-motion: reduce)',
    ).matches;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          inViewRef.current = true;
          observer.disconnect();
          startPlayback();
        }
      },
      { threshold: 0.35 },
    );
    observer.observe(wrapper);
    return () => observer.disconnect();
  }, [startPlayback]);

  useEffect(
    () => () => {
      for (const timer of timersRef.current) window.clearTimeout(timer);
    },
    [],
  );

  if (failed) {
    // wasm 加载失败的静态降级：同一段内容的纯文本（去 SGR），信息不丢
    return (
      <pre className="overflow-x-auto rounded-xl border border-fd-border bg-fd-card p-4 font-mono text-sm leading-relaxed">
        {fullText.replace(/\x1b\[[0-9;]*m/g, '')}
      </pre>
    );
  }

  return (
    <div
      ref={wrapRef}
      className="relative overflow-hidden rounded-xl border border-fd-border"
      // wasm 就绪前 wterm 只有 padding、行网格尚未建（行高 17px + 上下
      // padding 24px，与 @wterm/react 自动高度同式）——预占终态高度防跳
      style={{ minHeight: cast.header.height * 17 + 24 }}
      aria-busy={!wasmReady}
    >
      <Terminal
        ref={ref}
        cols={cast.header.width}
        rows={cast.header.height}
        wasmUrl={`${basePath}/wterm.wasm`}
        onData={() => {
          // 演示终端不吃键入；给出空回调同时关闭本地回显
        }}
        onReady={handleReady}
        onError={handleError}
        className="overflow-x-auto! [box-shadow:none]"
      />
      {!wasmReady && (
        <div
          className="absolute inset-0 flex items-center justify-center bg-[#1e1e1e]"
          aria-hidden="true"
        >
          <span className="animate-pulse font-mono text-sm text-[#d4d4d4]">
            终端加载中…
          </span>
        </div>
      )}
    </div>
  );
}
