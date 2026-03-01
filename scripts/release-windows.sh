#!/usr/bin/env bash
set -euo pipefail

# ── Sumi Windows Release Script (Git Bash) ──
# Run AFTER scripts/release.sh on macOS has created the GitHub Release.
# This builds Windows CUDA artifacts and uploads them to the existing release.
#
# Usage: bash scripts/release-windows.sh
#
# Prerequisites:
#   1. gh CLI installed and authenticated
#   2. Signing key at ~/.tauri/sumi.key
#   3. macOS release already created (tag + release exist on GitHub)
#   4. CUDA Toolkit, LLVM/Clang, Ninja, CMake, VS Build Tools

REPO="alan890104/sumi"
KEY_FILE="${HOME}/.tauri/sumi.key"

# ── Read current version ──
VERSION=$(python3 -c "import json; print(json.load(open('tauri.conf.json'))['version'])")
TAG="v${VERSION}"

echo "==> Windows release for ${TAG}"
echo ""

# ── Verify the GitHub release exists ──
if ! gh release view "$TAG" --repo "$REPO" &>/dev/null; then
    echo "ERROR: Release ${TAG} not found on GitHub."
    echo "Run scripts/release.sh on macOS first to create the release."
    exit 1
fi
echo "==> Found existing release ${TAG}"

# ── Check gh CLI ──
if ! command -v gh &> /dev/null; then
    echo "ERROR: gh CLI not found. Install from https://cli.github.com/"
    exit 1
fi

# ── Load signing key ──
if [ ! -f "$KEY_FILE" ]; then
    echo "ERROR: Signing key not found at ${KEY_FILE}"
    echo "Copy the key from your macOS machine or run:"
    echo "  cargo tauri signer generate -w ${KEY_FILE}"
    exit 1
fi

export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_FILE")"

echo -n "Signing key password: "
read -rs TAURI_SIGNING_PRIVATE_KEY_PASSWORD
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD
echo ""

# ── CUDA environment setup ──
export MSYS_NO_PATHCONV=1
export LIBCLANG_PATH="C:/Program Files/LLVM/bin"
export HOST_CMAKE_GENERATOR=Ninja

MSVC_DIR="$(ls -d "/c/Program Files (x86)/Microsoft Visual Studio"/*/BuildTools/VC/Tools/MSVC/*/ 2>/dev/null | sort -V | tail -1)"
if [ -z "$MSVC_DIR" ]; then
    echo "ERROR: Could not find VS Build Tools. Install Visual Studio Build Tools first." >&2
    exit 1
fi
export NVCC_CCBIN="${MSVC_DIR}bin/HostX64/x64/cl.exe"
export CMAKE_CUDA_FLAGS="--allow-unsupported-compiler -Xcompiler=-MT"
export CMAKE_CUDA_FLAGS_RELWITHDEBINFO='-Xcompiler="-MT -Zi -O2 -Ob1" -DNDEBUG'
export RUSTFLAGS="-C target-feature=+crt-static"
export CMAKE_C_FLAGS_RELWITHDEBINFO="-MT -Zi -O2 -Ob1 -DNDEBUG"
export CMAKE_CXX_FLAGS_RELWITHDEBINFO="-MT -Zi -O2 -Ob1 -DNDEBUG"
export CMAKE_C_FLAGS_DEBUG="-MT -Zi -Ob0 -Od"
export CMAKE_CXX_FLAGS_DEBUG="-MT -Zi -Ob0 -Od"
export CMAKE_C_FLAGS_RELEASE="-MT -O2 -Ob2 -DNDEBUG"
export CMAKE_CXX_FLAGS_RELEASE="-MT -O2 -Ob2 -DNDEBUG"

# ── Build ──
echo "==> Building Windows CUDA..."
cargo tauri build --no-default-features --features cuda

# ── Collect artifacts ──
ARTIFACTS=()
BUNDLE_DIR="target/release/bundle"

# MSI
for f in "${BUNDLE_DIR}/msi"/*.msi; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
done
for f in "${BUNDLE_DIR}/msi"/*.msi.zip; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
done
for f in "${BUNDLE_DIR}/msi"/*.msi.zip.sig; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
done

# NSIS
for f in "${BUNDLE_DIR}/nsis"/*-setup.exe; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
done
for f in "${BUNDLE_DIR}/nsis"/*.nsis.zip; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
done
for f in "${BUNDLE_DIR}/nsis"/*.nsis.zip.sig; do
    [ -f "$f" ] && ARTIFACTS+=("$f")
done

if [ ${#ARTIFACTS[@]} -eq 0 ]; then
    echo "ERROR: No build artifacts found in ${BUNDLE_DIR}"
    exit 1
fi

echo ""
echo "==> Build complete. Artifacts:"
printf '    %s\n' "${ARTIFACTS[@]}"

# ── Update latest.json with Windows platform ──
echo ""
echo "==> Updating latest.json..."

# Download existing latest.json from the release
gh release download "$TAG" --repo "$REPO" --pattern "latest.json" --clobber 2>/dev/null || echo '{}' > latest.json

for sig_file in "${ARTIFACTS[@]}"; do
    [[ "$sig_file" == *.sig ]] || continue
    archive="${sig_file%.sig}"
    archive_name="$(basename "$archive")"
    signature="$(cat "$sig_file")"
    url="https://github.com/${REPO}/releases/download/${TAG}/${archive_name}"

    # Use the MSI updater artifact for windows-x86_64
    if echo "$archive_name" | grep -qi "\.msi\.zip$"; then
        python3 -c "
import json, pathlib
p = pathlib.Path('latest.json')
d = json.loads(p.read_text())
d.setdefault('platforms', {})['windows-x86_64'] = {
    'url': '$url',
    'signature': '''$signature'''
}
p.write_text(json.dumps(d, indent=2) + '\n')
"
    fi
done

ARTIFACTS+=("latest.json")

echo ""
echo "==> Updated latest.json:"
cat latest.json

# ── Upload to existing release ──
echo ""
echo "==> Uploading to ${TAG}..."
gh release upload "$TAG" --repo "$REPO" --clobber "${ARTIFACTS[@]}"

echo ""
echo "==> Done! Release: https://github.com/${REPO}/releases/tag/${TAG}"

# Cleanup
rm -f latest.json
