#!/usr/bin/env bash
# 捕获 vbumpp 真实发版输出 → 生成首页终端演示素材 website/app/(home)/demo-terminal.ts
#
# 管道：临时 fixture（git 仓库 1.0.0 基线 + feat/fix 两提交 + bare 相对路径 remote，
#   .vbumpprc.toml 配 release = "minor" + confirm = false——COL-60 接通后全程非交互）
#   → pty 实跑（保 TTY 着色输出，零击键投喂）
#   → 原始字节流 → scripts/terminal-screen.mjs 塌缩 ANSI 重绘帧为最终屏幕
#   → 绝对路径洗白为 ~ → 导出为 TS 字符串常量
#
# 复跑确定性（验收：两次运行产物字节一致）：
#   - GIT_AUTHOR_DATE / GIT_COMMITTER_DATE 钉死 → 全部 commit/tag hash 恒定
#   - GIT_CONFIG_GLOBAL / GIT_CONFIG_SYSTEM=/dev/null 隔离宿主 git 配置（gpgsign 等）
#   - VBUMPP_HOME 指向 fixture 内目录，隔离全局 vbumpp 配置
#   - pty 尺寸钉 80x24（stty），不随捕获终端变化
#
# 依赖：macOS BSD script(1)（-q /dev/null 语法）、node、已构建的 target/release/vbumpp。
# 用法：website/scripts/capture-home-demo.sh（或 pnpm --filter website capture:home-demo）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEBSITE="$(dirname "$SCRIPT_DIR")"
ROOT="$(dirname "$WEBSITE")"
BINARY="$ROOT/target/release/vbumpp"
SCREEN_MJS="$SCRIPT_DIR/terminal-screen.mjs"
OUT_TS="$WEBSITE/app/(home)/demo-terminal.ts"

PIN_DATE='2026-08-05T10:00:00'
export GIT_AUTHOR_DATE="$PIN_DATE" GIT_COMMITTER_DATE="$PIN_DATE"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
# TERM 钉死：着色判定不随宿主终端（塌缩步骤忽略 SGR，产物不因此变化）
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

# ---- fixture：1.0.0 基线（tag v1.0.0）+ feat + fix，预推到本地 bare remote ----
# remote 用绝对路径（洗白后显示为 ~/my-project.git，避免 ../remote.git 这类
# fixture 内部痕迹出现在 push 行）；纯本地路径，push 离线即可成功
git init -q --bare "$WORK/my-project.git"
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
confirm = false

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
git remote add origin "$WORK_PHYSICAL/my-project.git"
git push -q -u origin main
git push -q origin v1.0.0

# ---- pty 实跑：配置已指定 release + confirm=false，全程无 prompt，零击键投喂
# （stdin 立空即可——macOS script(1) 在 stdin EOF 后仍跑到子进程退出）。
# pty 仍必要：dialoguer console::style 按 TTY 探测着色，演示要的就是彩色输出
RAW="$WORK/raw.txt"
script -q /dev/null sh -c 'stty cols 80 rows 24; exec "$1"' _ "$BINARY" < /dev/null > "$RAW" || true

# ---- 塌缩 ANSI 重绘帧 → 最终屏幕 ----
SCREEN="$WORK/screen.txt"
node "$SCREEN_MJS" "$RAW" "$SCREEN"
if grep -q 'Current version' "$SCREEN"; then
  echo 'error: 捕获出现交互菜单——release 配置键未生效（COL-60 回归？），应全程非交互' >&2
  cat "$SCREEN" >&2
  exit 1
fi
if ! grep -q 'Git push' "$SCREEN"; then
  echo 'error: 捕获未包含完整发版流程（缺 Git push），最终屏幕：' >&2
  cat "$SCREEN" >&2
  exit 1
fi

# ---- 绝对路径洗白（/private/tmp/... → ~，保留 my-project 段）----
WASHED="$WORK/washed.txt"
sed -e "s|$WORK_PHYSICAL|~|g" -e "s|$WORK|~|g" "$SCREEN" > "$WASHED"
if grep -q "$WORK_PHYSICAL" "$WASHED"; then
  echo "error: 洗白后仍残留绝对路径 $WORK_PHYSICAL" >&2
  exit 1
fi

# ---- 生成 TS 产物（模板字符串转义：\、`、${）----
{
  echo '// 本文件由 website/scripts/capture-home-demo.sh 生成，勿手改。'
  echo '// 内容：target/release/vbumpp 在临时 fixture 中的真实非交互发版输出'
  echo '//（.vbumpprc.toml 配 release = "minor" + confirm = false，COL-60），'
  echo '// pty 原始字节流经 terminal-screen.mjs 塌缩为最终屏幕，绝对路径已洗白为 ~。'
  echo '// 首行 `$ vbumpp` 为演示提示符（非捕获内容）；复跑脚本可字节级复现其余内容。'
  echo 'export const DEMO_TERMINAL = `$ vbumpp'
  sed -e 's/\\/\\\\/g' -e 's/`/\\`/g' -e 's/\${/\\${/g' "$WASHED"
  echo '`;'
} > "$OUT_TS"

echo "ok: $OUT_TS"
