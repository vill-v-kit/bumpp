#!/usr/bin/env bash
# 采集首页滚动演示（ADR-0036）四段 cast 时间线 → 生成 website/app/(home)/demo-casts.ts
#
# 四段（全部 dry-run / 只读形态、离线确定性、零远端 API mock）：
#   1. dry-run           —— `vbumpp --dry-run`：单包发版计划预览
#   2. recursive-dry-run —— `vbumpp -r --dry-run`：monorepo 整树计划
#      （fixture 为多包工作区，root 与一个叶子包 private，展示锁步语义——
#        private 不豁免，整树同一新版本）
#   3. release-dry-run   —— `vbumpp release 1.1.0 --dry-run --provider github`：
#      平台 Release 补发预览（token 来源、host/repo、tag、计划 HTTP 请求；
#      fixture 预置 v1.1.0 tag 与对应 changelog 版本节，token 命中预置加密
#      keyring —— source: token store，假 token）
#   4. token-list        —— `vbumpp token list`：加密 token 管理清单
#      （fixture 预置 keyring：provider 级键 + gitlab host 作用域键两种形态）
#
# 管道：各段临时 fixture（git 仓库、提交与 tag 全部钉日期）
#   → pty 实跑（保 TTY 着色输出，零击键投喂——dry-run / list 全程无 prompt）
#   → 原始字节流（含 SGR 颜色序列）×4 → scripts/raw-to-cast.ts 合并转
#     asciicast v2 兼容事件流，产出单个 TS 模块（采集侧只做格式转换，
#     屏幕仿真归渲染层）
#
# 复跑确定性（验收：两次运行产物字节一致）：
#   - GIT_AUTHOR_DATE / GIT_COMMITTER_DATE 钉死（epoch+时区内部格式，免宿主时区
#     解析差）→ 全部 commit/tag hash 恒定
#   - GIT_CONFIG_GLOBAL / GIT_CONFIG_SYSTEM=/dev/null 隔离宿主 git 配置（gpgsign 等）
#   - VBUMPP_HOME 指向 fixture 内目录，隔离全局 vbumpp 配置
#   - token 环境变量与 VBUMPP_TOKEN_STORE 一律 unset——release 段必须命中预置
#     keyring，宿主若恰好设了 GH_TOKEN 等会静默改写 token 来源
#   - keyring 由 scripts/make-demo-keyring.ts 钉密钥/IV 生成，字节恒定
#   - TERM 钉 xterm-256color——着色判定不随宿主终端（宿主 TERM=dumb 会产出无色流）
#   - pty 尺寸钉 80x24（stty），不随捕获终端变化
#   - 事件切分与时间戳由 raw-to-cast.ts 按内容确定，与采集耗时、分块时序无关
#
# 依赖：macOS BSD script(1)（-q /dev/null 语法）、node、已构建的 target/release/vbumpp。
# 用法：website/scripts/capture-home-demo-cast.sh（或 pnpm --filter website capture:home-demo-cast）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEBSITE="$(dirname "$SCRIPT_DIR")"
ROOT="$(dirname "$WEBSITE")"
BINARY="$ROOT/target/release/vbumpp"
CAST_TS="$SCRIPT_DIR/raw-to-cast.ts"
KEYRING_TS="$SCRIPT_DIR/make-demo-keyring.ts"
OUT_TS="$WEBSITE/app/(home)/demo-casts.ts"

# 钉死到 epoch+时区内部格式：裸 ISO 串会被 git 按宿主本地时区解析，commit
# 对象嵌入的 offset 随地变（本地 +0800 vs CI UTC → hash 漂移，v6.2.0 tag CI
# 首挂实证）。该瞬时是提交产物的出生值，改动须连产物一起重采集。
export GIT_AUTHOR_DATE='@1785895200 +0800' GIT_COMMITTER_DATE='@1785895200 +0800'
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
export TERM=xterm-256color
# 宿主 token 环境隔离（见头注释）
unset GH_TOKEN GITHUB_TOKEN GITLAB_TOKEN GITEE_TOKEN GITCODE_TOKEN VBUMPP_TOKEN_STORE

if [[ ! -x "$BINARY" ]]; then
  echo "error: $BINARY 不存在，先 cargo build --release -p vbumpp" >&2
  exit 1
fi

WORK="$(mktemp -d /tmp/vbumpp-demo-XXXXXX)"
trap 'rm -rf "$WORK"' EXIT
WORK_PHYSICAL="$(cd "$WORK" && pwd -P)"
export VBUMPP_HOME="$WORK/vbumpp-home"

# git fixture 公共初始化（宿主配置已被 GIT_CONFIG_* 隔离，user/sign 再钉一层）
init_repo() {
  git init -q -b main
  git config user.name 'you'
  git config user.email 'you@example.com'
  git config commit.gpgsign false
  git config tag.gpgsign false
}

# bump 两 fixture 共用的配置：minor 预演 + 中文 changelog 分组标题
write_vbumpprc() {
  cat > .vbumpprc.toml <<'EOF'
release = "minor"

[changelog.types.feat]
title = "🚀 特性"

[changelog.types.fix]
title = "🩹 修复"
EOF
}

# pty 实跑一段：$1=原始流输出 $2=工作目录 余下为 vbumpp 参数。
# stdin 立空即可——macOS script(1) 在 stdin EOF 后仍跑到子进程退出。
# pty 仍必要：console::style 按 TTY 探测着色，演示要的就是彩色输出
run_pty() {
  local raw="$1" cwd="$2"
  shift 2
  (cd "$cwd" && script -q /dev/null sh -c 'stty cols 80 rows 24; exec "$@"' _ "$BINARY" "$@" < /dev/null > "$raw") || true
}

# 完整性门禁：$1=原始流 $2=模式 $3=缺失说明；任一模式缺失即产物不可信
gate() {
  local raw="$1" pattern="$2" what="$3"
  if ! grep -q "$pattern" "$raw"; then
    echo "error: 捕获未包含${what}，原始流：" >&2
    cat -v "$raw" >&2
    exit 1
  fi
}

# ---- fixture 1：单包——1.0.0 基线（tag v1.0.0）+ feat + fix ----
SINGLE="$WORK/my-project"
mkdir -p "$SINGLE"
cd "$SINGLE"
init_repo

echo '{"name":"my-project","version":"1.0.0"}' > package.json
write_vbumpprc

git add -A
git commit -q -m 'chore: initial commit'
git tag v1.0.0
git commit -q --allow-empty -m 'feat: 新增夜间模式'
git commit -q --allow-empty -m 'fix: 修复导出时的编码错误'

# ---- fixture 2：monorepo——root(private) + core + web(private)，同基线 ----
# private 包照常见于工作区形态（root 必然 private、web 为内部应用）；锁步语义
# 的看点正在于 private 不豁免——三份清单同一新版本
MONO="$WORK/my-monorepo"
mkdir -p "$MONO/packages/core" "$MONO/packages/web"
cd "$MONO"
init_repo

echo '{"name":"my-monorepo","private":true,"version":"1.0.0"}' > package.json
printf 'packages:\n  - packages/*\n' > pnpm-workspace.yaml
echo '{"name":"@my/core","version":"1.0.0"}' > packages/core/package.json
echo '{"name":"@my/web","private":true,"version":"1.0.0"}' > packages/web/package.json
write_vbumpprc

git add -A
git commit -q -m 'chore: initial commit'
git tag v1.0.0
git commit -q --allow-empty -m 'feat: core 支持增量编译'
git commit -q --allow-empty -m 'fix: web 修复导出时的编码错误'

# ---- fixture 3：release 补发——v1.1.0 tag + 对应 changelog 版本节 ----
# repository 短形式（owner/repo → github.com）优先于 git remote 解析，
# 纯本地即可确定 host/repo，无需 remote
RELEASE="$WORK/my-release-project"
mkdir -p "$RELEASE"
cd "$RELEASE"
init_repo

echo '{"name":"my-project","version":"1.1.0","repository":"you/my-project"}' > package.json
cat > CHANGELOG.md <<'EOF'
# Changelog

## v1.1.0

### 🚀 特性

- 新增夜间模式

### 🩹 修复

- 修复导出时的编码错误
EOF

git add -A
git commit -q -m 'chore: release v1.1.0'
git tag v1.1.0

# ---- 预置加密 keyring（假 token）——段 3 的 token 来源与段 4 的清单内容 ----
node "$KEYRING_TS" "$VBUMPP_HOME"

# ---- 四段 pty 实跑（dry-run / list 无 prompt，零击键投喂）----
RAW_DRY="$WORK/raw-dry-run.txt"
RAW_REC="$WORK/raw-recursive.txt"
RAW_REL="$WORK/raw-release.txt"
RAW_TOK="$WORK/raw-token-list.txt"

run_pty "$RAW_DRY" "$SINGLE" --dry-run
run_pty "$RAW_REC" "$MONO" -r --dry-run
run_pty "$RAW_REL" "$RELEASE" release 1.1.0 --dry-run --provider github
run_pty "$RAW_TOK" "$WORK" token list

# ---- 完整性门禁：各段关键标识必须都在，否则产物不可信 ----
gate "$RAW_DRY" 'bump plan (dry run' ' dry-run 计划标识'
gate "$RAW_DRY" 'changelog preview:' ' changelog 预览（缺上一 tag？）'
gate "$RAW_REC" 'bump plan (dry run' ' 整树计划标识'
gate "$RAW_REC" 'packages/core/package.json: update' ' core 包预演判定'
gate "$RAW_REC" 'packages/web/package.json: update' ' web 包（private 锁步）预演判定'
gate "$RAW_REL" 'release plan (dry run' ' release 计划标识'
gate "$RAW_REL" 'token source: token store' ' token 来源（预置 keyring 未命中？）'
gate "$RAW_REL" 'requests:' ' 计划 HTTP 请求'
gate "$RAW_TOK" 'github' ' github 清单项'
gate "$RAW_TOK" 'gitlab (https://gitlab.com)' ' gitlab host 作用域清单项'

# ---- 格式转换：四段合并 → 洗白绝对路径 → asciicast v2 事件流 → TS 模块 ----
node "$CAST_TS" "$OUT_TS" 80 24 xterm-256color "$WORK_PHYSICAL" "$WORK" -- \
  dry-run 'vbumpp --dry-run' "$RAW_DRY" \
  recursive-dry-run 'vbumpp -r --dry-run' "$RAW_REC" \
  release-dry-run 'vbumpp release 1.1.0 --dry-run --provider github' "$RAW_REL" \
  token-list 'vbumpp token list' "$RAW_TOK"

echo "ok: $OUT_TS"
