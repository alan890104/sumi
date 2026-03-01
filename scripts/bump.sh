#!/usr/bin/env bash
set -euo pipefail

# ── Sumi Version Bump Script ──
# Usage: ./scripts/bump.sh <patch|minor|major>
#
# Bumps the version in tauri.conf.json and Cargo.toml,
# regenerates Cargo.lock, commits, and creates a git tag.
# Does NOT push — prints the command for you to run.

if [ $# -ne 1 ] || [[ ! "$1" =~ ^(patch|minor|major)$ ]]; then
  echo "Usage: $0 <patch|minor|major>"
  exit 1
fi
BUMP_TYPE="$1"

# ── Read current version ──
CURRENT=$(python3 -c "import json; print(json.load(open('tauri.conf.json'))['version'])")
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP_TYPE" in
  patch) VERSION="$MAJOR.$MINOR.$((PATCH + 1))" ;;
  minor) VERSION="$MAJOR.$((MINOR + 1)).0" ;;
  major) VERSION="$((MAJOR + 1)).0.0" ;;
esac

TAG="v${VERSION}"

echo "  ${CURRENT} → ${VERSION}  (tag: ${TAG})"
echo ""

# ── Update version in config files ──
python3 -c "
import json, pathlib
p = pathlib.Path('tauri.conf.json')
d = json.loads(p.read_text())
d['version'] = '${VERSION}'
p.write_text(json.dumps(d, indent=4) + '\n')
"

# Update Cargo.toml version (first occurrence under [package])
sed -i.bak "s/^version = \"${CURRENT}\"/version = \"${VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

echo "==> Version updated to ${VERSION}"

# ── Regenerate Cargo.lock ──
cargo check --quiet 2>/dev/null || cargo check
echo "==> Cargo.lock updated"

# ── Commit and tag ──
git add tauri.conf.json Cargo.toml Cargo.lock
git commit -m "release: v${VERSION}"
git tag "$TAG"

echo "==> Committed and tagged ${TAG}"
echo ""
echo "Now push to trigger the release pipeline:"
echo ""
echo "  git push origin main ${TAG}"
echo ""
