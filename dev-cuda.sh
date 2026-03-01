#!/bin/bash
# Windows CUDA development script for Sumi
# Requires: CUDA Toolkit, LLVM/Clang (for bindgen), Ninja, CMake, VS Build Tools
# Usage: bash dev-cuda.sh [--release]
#
# Environment notes:
# - MSYS_NO_PATHCONV: prevents Git Bash from mangling /MT → C:/Program Files/Git/MT
# - HOST_CMAKE_GENERATOR: cmake-rs reads this but whisper-rs-sys won't forward it as -D
#   (avoids the double-generator bug where CMAKE_GENERATOR gets forwarded by whisper-rs-sys)
# - NVCC_CCBIN: cudaforge auto-adds --allow-unsupported-compiler when this is set
# - CMAKE_CUDA_FLAGS: --allow-unsupported-compiler for whisper-rs-sys cmake + -Xcompiler=-MT
# - CMAKE_CUDA_FLAGS_RELWITHDEBINFO: overrides default -MD with -MT for CUDA host compiler
# - RUSTFLAGS crt-static: unifies all Rust/cc crate compilations to /MT (static CRT)
# - CMAKE_*_FLAGS_*: per-config overrides to ensure whisper-rs-sys cmake uses /MT everywhere

export MSYS_NO_PATHCONV=1
export LIBCLANG_PATH="C:/Program Files/LLVM/bin"
export HOST_CMAKE_GENERATOR=Ninja
# Auto-detect cl.exe from VS Build Tools (works across VS versions)
MSVC_DIR="$(ls -d "/c/Program Files (x86)/Microsoft Visual Studio"/*/BuildTools/VC/Tools/MSVC/*/ 2>/dev/null | sort -V | tail -1)"
if [ -z "$MSVC_DIR" ]; then
    echo "Error: Could not find VS Build Tools. Install Visual Studio Build Tools first." >&2
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

if [ "$1" = "--release" ]; then
    cargo tauri dev -- --no-default-features --features cuda --release
else
    cargo tauri dev -- --no-default-features --features cuda
fi
