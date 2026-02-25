#!/usr/bin/env bash
set -euo pipefail

# ── OpenTypeless Local Release Script ──
# Usage: ./scripts/release.sh
#
# Prerequisites:
#   1. gh CLI installed and authenticated (brew install gh && gh auth login)
#   2. Signing key generated:
#        cargo tauri signer generate -w ~/.tauri/opentypeless.key

REPO="alan890104/opentypeless"
KEY_FILE="${HOME}/.tauri/opentypeless.key"

# ── Read current version ──
CURRENT=$(python3 -c "import json; print(json.load(open('tauri.conf.json'))['version'])")
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

echo "Current version: ${CURRENT}"
echo ""
echo "Select bump type:"
echo "  0) current → $CURRENT  (re-release, overwrite existing)"
echo "  1) patch   → $MAJOR.$MINOR.$((PATCH + 1))"
echo "  2) minor   → $MAJOR.$((MINOR + 1)).0"
echo "  3) major   → $((MAJOR + 1)).0.0"
echo ""
echo -n "Choice [0/1/2/3]: "
read -r CHOICE

case "$CHOICE" in
  0) VERSION="$CURRENT" ;;
  1) VERSION="$MAJOR.$MINOR.$((PATCH + 1))" ;;
  2) VERSION="$MAJOR.$((MINOR + 1)).0" ;;
  3) VERSION="$((MAJOR + 1)).0.0" ;;
  *) echo "Invalid choice"; exit 1 ;;
esac

TAG="v${VERSION}"
RERELEASE=false

if [ "$VERSION" = "$CURRENT" ]; then
  RERELEASE=true
  echo ""
  echo "  Re-release ${VERSION}  (tag: ${TAG})"
  echo ""
  echo "This will rebuild and overwrite the existing release."
  echo -n "Proceed? [y/N]: "
else
  echo ""
  echo "  ${CURRENT} → ${VERSION}  (tag: ${TAG})"
  echo ""
  echo "This will update version in tauri.conf.json and Cargo.toml."
  echo -n "Proceed? [y/N]: "
fi
read -r CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
  echo "Aborted."
  exit 0
fi

if [ "$RERELEASE" = false ]; then
  # ── Update version in config files ──
  python3 -c "
import json, pathlib
for f in ['tauri.conf.json']:
    p = pathlib.Path(f)
    d = json.loads(p.read_text())
    d['version'] = '${VERSION}'
    p.write_text(json.dumps(d, indent=4) + '\n')
"
  sed -i '' "s/^version = \"${CURRENT}\"/version = \"${VERSION}\"/" Cargo.toml

  echo "==> Version updated to ${VERSION}"

  # ── Commit version bump ──
  git add tauri.conf.json Cargo.toml
  git commit -m "release: v${VERSION}"
  echo "==> Committed version bump"
fi
echo ""

# ── Load signing key from file ──
if [ ! -f "$KEY_FILE" ]; then
  echo "ERROR: Signing key not found at ${KEY_FILE}"
  echo "Run: cargo tauri signer generate -w ${KEY_FILE}"
  exit 1
fi

export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_FILE")"

# Prompt for password (not stored anywhere)
echo -n "Signing key password: "
read -rs TAURI_SIGNING_PRIVATE_KEY_PASSWORD
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD
echo ""

# ── Check gh CLI ──
if ! command -v gh &> /dev/null; then
  echo "ERROR: gh CLI not found. Install with: brew install gh"
  exit 1
fi

# ── Build both architectures ──
export MACOSX_DEPLOYMENT_TARGET="11.0"
export CMAKE_OSX_DEPLOYMENT_TARGET="11.0"
TARGETS=("aarch64-apple-darwin")
ARTIFACTS=()

for target in "${TARGETS[@]}"; do
  echo "==> Building for ${target}..."
  cargo tauri build --target "$target"

  BUNDLE_DIR="target/${target}/release/bundle/macos"

  # Find the .app.tar.gz, .sig, and .dmg files
  for f in "${BUNDLE_DIR}"/*.app.tar.gz; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
  done
  for f in "${BUNDLE_DIR}"/*.app.tar.gz.sig; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
  done
  DMG_DIR="target/${target}/release/bundle/dmg"
  for f in "${DMG_DIR}"/*.dmg; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
  done
done

echo ""
echo "==> Build complete. Artifacts:"
printf '    %s\n' "${ARTIFACTS[@]}"

# ── Generate latest.json ──
DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
PLATFORMS='{}'

for sig_file in "${ARTIFACTS[@]}"; do
  [[ "$sig_file" == *.sig ]] || continue
  archive="${sig_file%.sig}"
  archive_name="$(basename "$archive")"
  signature="$(cat "$sig_file")"
  url="https://github.com/${REPO}/releases/download/${TAG}/${archive_name}"

  if echo "$sig_file" | grep -q "aarch64"; then
    PLATFORMS=$(echo "$PLATFORMS" | python3 -c "
import sys, json
d = json.load(sys.stdin)
d['darwin-aarch64'] = {'url': '$url', 'signature': '''$signature'''}
json.dump(d, sys.stdout)")
  elif echo "$sig_file" | grep -q "x86_64"; then
    PLATFORMS=$(echo "$PLATFORMS" | python3 -c "
import sys, json
d = json.load(sys.stdin)
d['darwin-x86_64'] = {'url': '$url', 'signature': '''$signature'''}
json.dump(d, sys.stdout)")
  fi
done

LATEST_JSON=$(python3 -c "
import json
print(json.dumps({
    'version': '${VERSION}',
    'pub_date': '${DATE}',
    'platforms': json.loads('${PLATFORMS}' if '${PLATFORMS}' else '{}')
}, indent=2))")

echo "$LATEST_JSON" > latest.json
ARTIFACTS+=("latest.json")

echo ""
echo "==> latest.json:"
cat latest.json

# ── Create GitHub Release ──
echo ""
echo "==> Creating GitHub Release ${TAG}..."

# Handle existing tag/release for re-release
if [ "$RERELEASE" = true ]; then
  # Delete existing release if it exists
  gh release delete "$TAG" --repo "$REPO" --yes 2>/dev/null || true
  # Move tag to current commit
  git tag -f "$TAG"
  git push origin "$TAG" --force
else
  git tag "$TAG"
  git push origin "$TAG"
fi

gh release create "$TAG" \
  --repo "$REPO" \
  --title "$TAG" \
  --generate-notes \
  "${ARTIFACTS[@]}"

echo ""
echo "==> Done! Release: https://github.com/${REPO}/releases/tag/${TAG}"

# Cleanup
rm -f latest.json
