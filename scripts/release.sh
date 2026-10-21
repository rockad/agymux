#!/usr/bin/env bash
set -euo pipefail

# Determine repository root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Check for uncommitted changes (excluding untracked files)
if ! git diff-index --quiet HEAD --; then
    echo "Error: Working directory has uncommitted changes. Please commit or stash them first." >&2
    exit 1
fi

# Determine next version
if [ -n "${1:-}" ]; then
    NEXT_VERSION="${1#v}"
else
    # Automatically compute semver bump using git-cliff from conventional commits
    if command -v git-cliff >/dev/null 2>&1; then
        CLIFF_CMD="git-cliff"
    elif command -v bun >/dev/null 2>&1; then
        CLIFF_CMD="bun x git-cliff"
    else
        echo "Error: Neither git-cliff nor bun found to compute semver bump." >&2
        exit 1
    fi

    NEXT_TAG="$($CLIFF_CMD --bumped-version)"
    NEXT_VERSION="${NEXT_TAG#v}"
fi

CURRENT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"

if [ "$CURRENT_VERSION" = "$NEXT_VERSION" ]; then
    echo "Current version is already $CURRENT_VERSION. No bump needed."
    exit 0
fi

echo "Bumping agymux release: v$CURRENT_VERSION -> v$NEXT_VERSION"

# 1. Update Cargo.toml
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEXT_VERSION\"/" Cargo.toml

# 2. Update Cargo.lock
cargo check --quiet

# 3. Generate CHANGELOG.md via git-cliff
if command -v git-cliff >/dev/null 2>&1; then
    git-cliff --tag "v$NEXT_VERSION" --output CHANGELOG.md
elif command -v bun >/dev/null 2>&1; then
    bun x git-cliff --tag "v$NEXT_VERSION" --output CHANGELOG.md
fi

# 4. Commit and Tag
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): bump version to $NEXT_VERSION"
git tag -a "v$NEXT_VERSION" -m "Release v$NEXT_VERSION"

echo "==> Successfully released v$NEXT_VERSION!"
echo "==> Tagged v$NEXT_VERSION"
echo "==> Push with: git push origin main --tags"
