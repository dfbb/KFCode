#!/usr/bin/env bash
# 发版入口:校验 -> 计算/更新版本号 -> 提交 -> 打 tag -> push。
#
# 用法:
#   bash scripts/build/release.sh            # 自动递增到下一个版本(推荐)
#   bash scripts/build/release.sh 0.3.0      # 显式指定版本(覆盖自动递增)
#
# 自动递增规则(每位逢 10 进 1,非标准 semver,但符合本项目约定):
#   首发 0.1.0 -> 0.1.1;之后 patch 每次 +1;patch 到 9 进位:0.1.9 -> 0.2.0;
#   minor 到 9 再进位:0.9.9 -> 1.0.0。基准取「Cargo.toml 版本」与「最新 git tag」较大者。
#
# push tag 后,GitHub Actions(.github/workflows/release.yml)会自动:
#   编译三平台二进制、打包、创建 GitHub Release、把 formula 推到 dfbb/homebrew-tap。
#
# 注:release.yml 的 tag 触发器由 dist 生成、匹配 vX.Y.Z,不要手改
#     (check-dist.sh 会用 `dist generate --check` 校验,手改会导致 CI 失败)。

set -euo pipefail
cd "$(dirname "$0")/../.."

# 每位逢 10 进 1 的递增:0.1.9 -> 0.2.0,0.9.9 -> 1.0.0
bump_version() {
  local major minor patch
  IFS='.' read -r major minor patch <<< "$1"
  patch=$((patch + 1))
  if (( patch > 9 )); then patch=0; minor=$((minor + 1)); fi
  if (( minor > 9 )); then minor=0; major=$((major + 1)); fi
  echo "${major}.${minor}.${patch}"
}

# 从 Cargo.toml 的 [workspace.package] 读当前版本
current_cargo_version() {
  perl -0ne 'print $1 if /\[workspace\.package\][^\[]*?\nversion = "([^"]+)"/s' Cargo.toml
}

# 最新的 vX.Y.Z tag(去掉 v 前缀);没有则空
# 注:grep 无匹配时返回非零,在 pipefail 下会使整条管道失败,故用 `|| true` 兜底
latest_tag_version() {
  git tag --list 'v[0-9]*.[0-9]*.[0-9]*' 2>/dev/null \
    | sed 's/^v//' \
    | { grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' || true; } \
    | sort -V | tail -1
}

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  # 自动递增:基准 = max(Cargo.toml 版本, 最新 tag),再 +1
  CARGO_V="$(current_cargo_version)"
  TAG_V="$(latest_tag_version)"
  BASE="$(printf '%s\n%s\n' "${CARGO_V:-0.0.0}" "${TAG_V:-0.0.0}" | sort -V | tail -1)"
  VERSION="$(bump_version "$BASE")"
  echo "==> 自动递增版本:基准 $BASE -> 新版本 $VERSION"
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

echo "OK: 已推送 ${TAG}。GitHub Actions 将自动编译三平台并发布 Release / 更新 Homebrew tap。"
echo "    查看进度: https://github.com/dfbb/KFCode/actions"
