#!/usr/bin/env bash
# 本地为当前平台构建并打包,产出压缩包供本地试装(不发布、不触碰 git)。
# 用法: bash scripts/build/package-local.sh
# 产物位于 target/distrib/

set -euo pipefail
cd "$(dirname "$0")/../.."

if ! command -v dist >/dev/null 2>&1; then
  echo "error: 'dist' 未安装。安装命令见 scripts/build/README.md" >&2
  exit 1
fi

echo "==> 为当前主机平台构建本地产物(archive)"
# 只构建当前主机 target,避免在本地尝试交叉编译其它平台
# (CI 中每个 target 在各自原生 runner 上构建,无需交叉工具链)。
HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
echo "    host target = $HOST_TARGET"
dist build --artifacts=local --target "$HOST_TARGET"

echo
echo "==> 产物输出目录:"
ls -lh target/distrib/ 2>/dev/null || echo "(未找到 target/distrib,请检查上方 dist 输出)"

echo "OK: 本地打包完成。"
