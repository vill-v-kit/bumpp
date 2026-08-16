'use client';

// 首页滚动演示区（COL-92，ADR-0036 的最终形态）：sticky wterm 终端随滚动
// 依次演示四段子命令，侧列卖点文案随当前步骤高亮。滚动进度经
// demo-scrub.ts 定位到「段落 + 事件数」，重放式 seek 由 ReplayCore
// 完成（重置核心 + 同步写入 ≤t 字节），同段向前滚动只增量补写。
// 降级：lg 断点以下与 prefers-reduced-motion 渲染每步一张静态终态卡片。

import { Globe, Layers, ShieldCheck, Zap } from 'lucide-react';
import { useMotionValueEvent, useScroll } from 'motion/react';
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from 'react';
import type { WTerm } from '@wterm/react';

import { cn } from '@/lib/cn';
import { DEMO_SEGMENTS } from './demo-casts';
import {
  buildTimeline,
  locateAt,
  planScrub,
  segmentText,
  type ScrubTarget,
} from './demo-scrub';
import { DemoTerminal, StaticCastCard, TerminalCard, useReplayCore } from './demo-terminal-frame';

interface DemoStep {
  readonly icon: ReactNode;
  readonly title: string;
  readonly desc: string;
}

// 四步卖点文案——与 DEMO_SEGMENTS 同序；原「优势」六卡的文案并入对应步骤
const STEPS: readonly DemoStep[] = [
  {
    icon: <Zap className="size-4" />,
    title: '一条命令发版，dry-run 先看清',
    desc: '版本号、commit / tag / push 一次完成，CHANGELOG 从 conventional commits 自动生成；--dry-run 列出完整计划与 changelog 预览，不改任何文件',
  },
  {
    icon: <Layers className="size-4" />,
    title: 'monorepo 整树递归',
    desc: '一个 -r 同步更新所有包的版本号（private 包也一并更新）；package.json、Cargo.toml 等清单结构化更新，lockfile 也可自动刷新',
  },
  {
    icon: <Globe className="size-4" />,
    title: '四家平台 Release',
    desc: 'GitHub / GitLab / Gitee / GitCode 自动创建，失败可单独补发重试；--dry-run 先预览 token 来源与请求计划',
  },
  {
    icon: <ShieldCheck className="size-4" />,
    title: 'token 加密存储',
    desc: '一次录入本机加密保存，按 provider / host 作用域管理；CI 环境直接用各家环境变量',
  },
];

/** SSR/水合期用服务端快照，水合后取真实匹配值（避免 hydration 不一致） */
function useMediaQuery(query: string, serverValue: boolean): boolean {
  return useSyncExternalStore(
    (callback) => {
      const mq = window.matchMedia(query);
      mq.addEventListener('change', callback);
      return () => mq.removeEventListener('change', callback);
    },
    () => window.matchMedia(query).matches,
    () => serverValue,
  );
}

export function ScrollyDemo() {
  // 服务端快照按桌面交互形态渲染（流量主体）；移动端/减少动态效果在水合后
  // 换成等信息的静态终态卡片
  const desktop = useMediaQuery('(min-width: 1024px)', true);
  const reducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)', false);
  if (!desktop || reducedMotion) return <StaticDemoCards />;
  return <ScrollySection />;
}

function StaticDemoCards() {
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-12 px-6 pb-16">
      {STEPS.map((step, i) => (
        <div key={DEMO_SEGMENTS[i].id}>
          <div className="mb-2 flex items-center gap-2 font-medium">
            <span className="text-fd-primary">{step.icon}</span>
            {step.title}
          </div>
          <p className="mb-4 text-sm text-fd-muted-foreground">{step.desc}</p>
          <StaticCastCard segment={DEMO_SEGMENTS[i]} />
        </div>
      ))}
    </div>
  );
}

function ScrollySection() {
  const sectionRef = useRef<HTMLElement | null>(null);
  const { core, failed } = useReplayCore();
  const [initFailed, setInitFailed] = useState(false);
  const wtRef = useRef<WTerm | null>(null);
  // 已应用到终端的位置（就绪/重挂载时按 scrollYProgress 现算目标）
  const appliedRef = useRef<ScrubTarget | null>(null);
  const rafRef = useRef(0);
  const [activeStep, setActiveStep] = useState(0);
  const timeline = useMemo(() => buildTimeline(DEMO_SEGMENTS), []);

  const { scrollYProgress } = useScroll({
    target: sectionRef,
    offset: ['start start', 'end end'],
  });

  const apply = useCallback(
    (target: ScrubTarget) => {
      const wt = wtRef.current;
      if (!wt || !core) return;
      const plan = planScrub(appliedRef.current, target);
      if (plan.kind === 'none') return;
      const cast = DEMO_SEGMENTS[plan.segmentIndex].cast;
      if (plan.kind === 'replay') {
        core.replay(segmentText(cast, 0, plan.to));
        // write 是 WTerm 唯一会排 rAF 重绘的公开入口：空写触发一次绘制，
        // 渲染层随即读到重放后的屏幕状态
        wt.write('');
      } else {
        wt.write(segmentText(cast, plan.from, plan.to));
      }
      appliedRef.current = target;
      setActiveStep(target.segmentIndex);
    },
    [core],
  );

  // 滚动事件逐帧合并：一帧内多次 progress 变化只应用最新目标
  useMotionValueEvent(scrollYProgress, 'change', (p) => {
    const target = locateAt(p, timeline);
    cancelAnimationFrame(rafRef.current);
    rafRef.current = requestAnimationFrame(() => apply(target));
  });

  useEffect(() => () => cancelAnimationFrame(rafRef.current), []);

  const handleReady = useCallback(
    (wt: WTerm) => {
      // wterm init 会抢焦点，页面场景还给文档流（键盘滚动、tab 序不被终端截走）
      wt.element.blur();
      wtRef.current = wt;
      // dev 严格模式重挂载会销毁重建终端：已应用状态作废，按当前进度整段重放
      appliedRef.current = null;
      apply(locateAt(scrollYProgress.get(), timeline));
    },
    [apply, scrollYProgress, timeline],
  );

  const handleError = useCallback(() => setInitFailed(true), []);

  if (failed || initFailed) return <StaticDemoCards />;

  return (
    <section
      ref={sectionRef}
      aria-label="子命令演示"
      className="relative h-[400vh]"
    >
      <div className="sticky top-14 flex h-[calc(100vh-3.5rem)] items-center overflow-hidden">
        <div className="mx-auto grid w-full max-w-6xl items-center gap-10 px-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
          <ol className="flex flex-col gap-6">
            {STEPS.map((step, i) => {
              const active = i === activeStep;
              return (
                <li
                  key={DEMO_SEGMENTS[i].id}
                  className={cn(
                    'border-l-2 pl-4 transition-colors duration-200',
                    active ? 'border-fd-primary' : 'border-fd-border',
                  )}
                >
                  <div
                    className={cn(
                      'flex items-center gap-2 font-medium transition-colors duration-200',
                      active ? 'text-fd-foreground' : 'text-fd-muted-foreground',
                    )}
                  >
                    <span className={active ? 'text-fd-primary' : undefined}>
                      {step.icon}
                    </span>
                    {step.title}
                  </div>
                  <p
                    className={cn(
                      'mt-1 text-sm text-fd-muted-foreground transition-opacity duration-200',
                      active ? 'opacity-100' : 'opacity-60',
                    )}
                  >
                    {step.desc}
                  </p>
                </li>
              );
            })}
          </ol>
          <TerminalCard title={`$ ${DEMO_SEGMENTS[activeStep].command}`}>
            <DemoTerminal
              core={core}
              cast={DEMO_SEGMENTS[activeStep].cast}
              onReady={handleReady}
              onError={handleError}
            />
          </TerminalCard>
        </div>
      </div>
    </section>
  );
}
