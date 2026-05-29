#!/usr/bin/env bash
# 发版入口:校验 -> 更新 workspace 版本号 -> 提交 -> 打 tag -> push。
# 用法: bash scripts/build/release.sh <version>   例如: bash scripts/build/release.sh 0.1.1
#
# push tag 后,GitHub Actions(.github/workflows/release.yml)会自动:
#   编译三平台二进制、打包、创建 GitHub Release、把 formula 推到 dfbb/homebrew-tap。

set -euo pipefail
cd "$(dirname "$0")/../.."

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  echo "用法: bash scripts/build/release.sh <version>   例如 0.1.1" >&2
  exit 1
fi

# 校验版本号格式:X.Y.Z(可带 -prerelease 后缀)
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "error: 版本号格式非法: '$VERSION'(期望 X.Y.Z 或 X.Y.Z-suffix)" >&2
  exit 1
fi

TAG="v$VERSION"

# 必须在干净的工作区
if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: 工作区不干净,请先提交或暂存改动。" >&2
  git status --short >&2
  exit 1
fi

# tag 不能已存在
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "error: tag '$TAG' 已存在。" >&2
  exit 1
fi

# 发版前自检(配置与工作流一致 + 产物计划)
echo "==> 运行 check-dist.sh 自检"
bash scripts/build/check-dist.sh

# 更新 [workspace.package] version
echo "==> 更新 Cargo.toml 版本号为 $VERSION"
# 仅替换 [workspace.package] 段内的第一处 version 行
perl -0pi -e 's/(\[workspace\.package\][^\[]*?\nversion = ")[^"]+(")/${1}'"$VERSION"'${2}/s' Cargo.toml

# 刷新 Cargo.lock 中本工作区包的版本
cargo update --workspace --offline 2>/dev/null || cargo update --workspace || true

# 确认确实改动了版本
if ! grep -q "version = \"$VERSION\"" Cargo.toml; then
  echo "error: 版本号更新失败,请手动检查 Cargo.toml。" >&2
  exit 1
fi

echo "==> 提交并打 tag $TAG"
git add Cargo.toml Cargo.lock
git commit -m "release: $VERSION"
git tag -a "$TAG" -m "release $VERSION"

echo "==> 推送 commit 与 tag"
git push
git push origin "$TAG"

echo "OK: 已推送 $TAG。GitHub Actions 将自动编译三平台并发布 Release / 更新 Homebrew tap。"
echo "    查看进度: https://github.com/dfbb/KFCode/actions"
