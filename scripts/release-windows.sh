#!/usr/bin/env bash
set -euo pipefail

# ── Sumi Windows Release Script (Git Bash) ──
# Run AFTER scripts/release.sh on macOS has created the GitHub Release.
# This builds Windows CPU + CUDA MSI artifacts and uploads them to the existing release.
#
# Usage: bash scripts/release-windows.sh
#
# Prerequisites:
#   1. gh CLI installed and authenticated
#   2. Signing key at ~/.tauri/sumi.key
#   3. macOS release already created (tag + release exist on GitHub)
#   4. CUDA Toolkit, LLVM/Clang, Ninja, CMake, VS Build Tools
#   5. Python 3 (for reading version from tauri.conf.json)

REPO="alan890104/sumi"
KEY_FILE="${HOME}/.tauri/sumi.key"

# ── Read current version ──
VERSION=$(python3 -c "import json; print(json.load(open('tauri.conf.json'))['version'])")
TAG="v${VERSION}"

echo "==> Windows release for ${TAG}"
echo ""

# ── Check gh CLI ──
if ! command -v gh &> /dev/null; then
    echo "ERROR: gh CLI not found. Install from https://cli.github.com/"
    exit 1
fi

# ── Verify the GitHub release exists ──
if ! gh release view "$TAG" --repo "$REPO" &>/dev/null; then
    echo "ERROR: Release ${TAG} not found on GitHub."
    echo "Run scripts/release.sh on macOS first to create the release."
    exit 1
fi
echo "==> Found existing release ${TAG}"

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

# ── Verify password before building ──
echo "==> Verifying signing key password..."
TEMP_SIGN_FILE="$(mktemp)"
echo "test" > "$TEMP_SIGN_FILE"
if ! cargo tauri signer sign "$TEMP_SIGN_FILE" 2>/dev/null; then
    rm -f "$TEMP_SIGN_FILE" "${TEMP_SIGN_FILE}.sig"
    echo "ERROR: Wrong signing key password. Aborting."
    exit 1
fi
rm -f "$TEMP_SIGN_FILE" "${TEMP_SIGN_FILE}.sig"
echo "==> Signing key verified."

# ── Common environment setup ──
export MSYS_NO_PATHCONV=1
export LIBCLANG_PATH="C:/Program Files/LLVM/bin"
export RUSTFLAGS="-C target-feature=+crt-static"

# ── Staging directory for renamed artifacts ──
STAGING="target/release/staging"
rm -rf "$STAGING"
mkdir -p "$STAGING"

BUNDLE_DIR="target/release/bundle"

# ── Helper: collect and rename MSI artifacts ──
# Usage: collect_artifacts <suffix>
# Copies MSI artifacts to staging with the given suffix (e.g. "_cpu", "_cuda")
collect_artifacts() {
    local suffix="$1"

    # Clean old bundle artifacts to avoid mixing versions
    # (cargo tauri build doesn't always clean previous output)

    # Strip WiX locale suffix (e.g. "_en-US") from filenames — it only
    # describes the installer UI language, not the app's supported languages.
    strip_locale() { echo "$1" | sed 's/_en-US//'; }

    # MSI installer + updater signature (Tauri v2 format: .msi + .msi.sig)
    for f in "${BUNDLE_DIR}/msi"/*.msi; do
        [[ "$f" == *.msi.sig ]] && continue
        [ -f "$f" ] || continue
        local base="$(strip_locale "$(basename "$f" .msi)")"
        cp "$f" "${STAGING}/${base}${suffix}.msi"
        [ -f "$f.sig" ] && cp "$f.sig" "${STAGING}/${base}${suffix}.msi.sig"
    done
}

# ══════════════════════════════════════════
# Build 1: CPU-only (no CUDA, no Metal)
# ══════════════════════════════════════════
echo ""
echo "==> [1/2] Building Windows CPU-only..."

# Clean previous bundle output to avoid version mismatch
rm -rf "${BUNDLE_DIR}/msi"

cargo tauri build -- --no-default-features

collect_artifacts "_cpu"
echo "==> CPU build artifacts collected."

# ══════════════════════════════════════════
# Build 2: CUDA
# ══════════════════════════════════════════
echo ""
echo "==> [2/2] Building Windows CUDA..."

# Clean previous bundle output
rm -rf "${BUNDLE_DIR}/msi"

# CUDA-specific environment
export HOST_CMAKE_GENERATOR=Ninja
MSVC_DIR="$(ls -d "/c/Program Files (x86)/Microsoft Visual Studio"/*/BuildTools/VC/Tools/MSVC/*/ 2>/dev/null | sort -V | tail -1)"
if [ -z "$MSVC_DIR" ]; then
    echo "ERROR: Could not find VS Build Tools." >&2
    exit 1
fi
MSVC_DIR="$(cygpath -m "$MSVC_DIR")"
export NVCC_CCBIN="${MSVC_DIR}bin/HostX64/x64/cl.exe"
export NVCC_PREPEND_FLAGS="--allow-unsupported-compiler"
export CMAKE_CUDA_FLAGS="--allow-unsupported-compiler -Xcompiler=-MT"
export CMAKE_CUDA_FLAGS_RELEASE='-Xcompiler="-MT -O2 -Ob2" -DNDEBUG'
export CMAKE_CUDA_FLAGS_RELWITHDEBINFO='-Xcompiler="-MT -Zi -O2 -Ob1" -DNDEBUG'
export CMAKE_C_FLAGS_RELWITHDEBINFO="-MT -Zi -O2 -Ob1 -DNDEBUG"
export CMAKE_CXX_FLAGS_RELWITHDEBINFO="-MT -Zi -O2 -Ob1 -DNDEBUG"
export CMAKE_C_FLAGS_DEBUG="-MT -Zi -Ob0 -Od"
export CMAKE_CXX_FLAGS_DEBUG="-MT -Zi -Ob0 -Od"
export CMAKE_C_FLAGS_RELEASE="-MT -O2 -Ob2 -DNDEBUG"
export CMAKE_CXX_FLAGS_RELEASE="-MT -O2 -Ob2 -DNDEBUG"

cargo tauri build -- --no-default-features --features cuda

collect_artifacts "_cuda"
echo "==> CUDA build artifacts collected."

# ── List all staged artifacts ──
echo ""
echo "==> All artifacts:"
ls -lh "$STAGING"/
ARTIFACTS=("${STAGING}"/*)

# ── Update latest.json with Windows platform ──
# Use the CPU MSI updater for auto-update (universal, works without CUDA)
echo ""
echo "==> Updating latest.json..."

gh release download "$TAG" --repo "$REPO" --pattern "latest.json" --clobber 2>/dev/null || echo '{}' > latest.json

for sig_file in "${STAGING}"/*_cpu.msi.sig; do
    [ -f "$sig_file" ] || continue
    msi_file="${sig_file%.sig}"
    msi_name="$(basename "$msi_file")"
    signature="$(cat "$sig_file")"
    url="https://github.com/${REPO}/releases/download/${TAG}/${msi_name}"

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
