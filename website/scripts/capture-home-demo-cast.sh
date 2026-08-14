#!/usr/bin/env bash
# 采集首页滚动演示（ADR-0036）cast 时间线 → 生成 website/app/(home)/demo-casts.ts
#
# 管道：临时 fixture（git 仓库 1.0.0 基线 + feat/fix 两提交；dry-run 全程只读、
#   零副作用，故无需 bare remote）
#   → pty 实跑 `vbumpp --dry-run`（保 TTY 着色输出，零击键投喂）
#   → 原始字节流（含 SGR 颜色序列）→ scripts/raw-to-cast.mjs 转 asciicast v2
#     兼容事件流（采集侧只做格式转换，屏幕仿真归渲染层）
#   → 导出为 TS 模块——与现存的静态最终屏 demo-terminal.ts 并存，本票不动首页渲染
#
# 复跑确定性（验收：两次运行产物字节一致）：
#   - GIT_AUTHOR_DATE / GIT_COMMITTER_DATE 钉死 → 全部 commit/tag hash 恒定
#   - GIT_CONFIG_GLOBAL / GIT_CONFIG_SYSTEM=/dev/null 隔离宿主 git 配置（gpgsign 等）
#   - VBUMPP_HOME 指向 fixture 内目录，隔离全局 vbumpp 配置
#   - TERM 钉 xterm-256color——着色判定不随宿主终端（宿主 TERM=dumb 会产出无色流）
#   - pty 尺寸钉 80x24（stty），不随捕获终端变化
#   - 事件切分与时间戳由 raw-to-cast.mjs 按内容确定，与采集耗时、分块时序无关
#
# 依赖：macOS BSD script(1)（-q /dev/null 语法）、node、已构建的 target/release/vbumpp。
# 用法：website/scripts/capture-home-demo-cast.sh（或 pnpm --filter website capture:home-demo-cast）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEBSITE="$(dirname "$SCRIPT_DIR")"
ROOT="$(dirname "$WEBSITE")"
BINARY="$ROOT/target/release/vbumpp"
CAST_MJS="$SCRIPT_DIR/raw-to-cast.mjs"
OUT_TS="$WEBSITE/app/(home)/demo-casts.ts"

PIN_DATE='2026-08-05T10:00:00'
export GIT_AUTHOR_DATE="$PIN_DATE" GIT_COMMITTER_DATE="$PIN_DATE"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
export TERM=xterm-256color

if [[ ! -x "$BINARY" ]]; then
  echo "error: $BINARY 不存在，先 cargo build --release -p vbumpp" >&2
  exit 1
fi

WORK="$(mktemp -d /tmp/vbumpp-demo-XXXXXX)"
trap 'rm -rf "$WORK"' EXIT
WORK_PHYSICAL="$(cd "$WORK" && pwd -P)"
PROJECT="$WORK/my-project"
export VBUMPP_HOME="$WORK/vbumpp-home"

# ---- fixture：1.0.0 基线（tag v1.0.0）+ feat + fix ----
mkdir -p "$PROJECT"
cd "$PROJECT"
git init -q -b main
git config user.name 'you'
git config user.email 'you@example.com'
git config commit.gpgsign false
git config tag.gpgsign false

echo '{"name":"my-project","version":"1.0.0"}' > package.json
cat > .vbumpprc.toml <<'EOF'
release = "minor"

[changelog.types.feat]
title = "🚀 特性"

[changelog.types.fix]
title = "🩹 修复"
EOF

git add -A
git commit -q -m 'chore: initial commit'
git tag v1.0.0
git commit -q --allow-empty -m 'feat: 新增夜间模式'
git commit -q --allow-empty -m 'fix: 修复导出时的编码错误'

# ---- pty 实跑：dry-run 无 prompt，零击键投喂（stdin 立空即可——macOS
# script(1) 在 stdin EOF 后仍跑到子进程退出）。pty 仍必要：console::style
# 按 TTY 探测着色，演示要的就是彩色输出
RAW="$WORK/raw.txt"
script -q /dev/null sh -c 'stty cols 80 rows 24; exec "$1" --dry-run' _ "$BINARY" < /dev/null > "$RAW" || true

# ---- 完整性门禁：计划标识与 changelog 预览必须都在，否则产物不可信 ----
if ! grep -q 'bump plan (dry run' "$RAW"; then
  echo 'error: 捕获未包含 dry-run 计划标识，原始流：' >&2
  cat -v "$RAW" >&2
  exit 1
fi
if ! grep -q 'changelog preview:' "$RAW"; then
  echo 'error: 捕获未包含 changelog 预览（缺上一 tag？），原始流：' >&2
  cat -v "$RAW" >&2
  exit 1
fi

# ---- 格式转换：洗白绝对路径 → asciicast v2 事件流 → TS 模块 ----
node "$CAST_MJS" "$RAW" "$OUT_TS" 'vbumpp --dry-run' 80 24 xterm-256color "$WORK_PHYSICAL" "$WORK"

echo "ok: $OUT_TS"
