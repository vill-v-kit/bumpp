/**
 * vbumpp 品牌标：主题色 #ff6736 圆徽（水波 = 发版进度条隐喻，v 弱化并在
 * 水线处被波面截断）。
 *
 * 以 <img> 引用独立 logo.svg——文档页存在两处 logo 实例（桌面侧栏 +
 * 移动端菜单头），内联 SVG 的 defs id 会互相串 paint server（渲染降级），
 * 独立文档天然隔离，且浏览器只缓存一份
 */
import { basePath } from '@/lib/shared';

export function Logo({ className }: { className?: string }) {
  return <img src={`${basePath}/logo.svg`} alt="vbumpp logo" className={className} />;
}
