#!/usr/bin/env bash
# CI 前自检:校验 dist 配置与已生成的工作流是否一致,并打印发布产物计划。
# 用法: bash scripts/build/check-dist.sh

set -euo pipefail
cd "$(dirname "$0")/../.."

if ! command -v dist >/dev/null 2>&1; then
  echo "error: 'dist' 未安装。安装命令见 scripts/build/README.md" >&2
  exit 1
fi

echo "==> dist 版本"
dist --version

echo "==> 校验 .github/workflows/release.yml 与配置一致(generate --check)"
dist generate --check

echo "==> 发布产物计划(dist plan)"
dist plan

echo "OK: dist 配置与工作流一致。"
