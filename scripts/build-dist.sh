#!/usr/bin/env bash
set -euo pipefail

build() (
    local platform=$1 target=$2 suffix=${3:-}
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

    RUSTC=$(rustup which --toolchain stable rustc)
    export RUSTC
    rustup target add --toolchain stable "$target"
    rustup run stable cargo build --workspace --bins --release \
        --target "$target" --target-dir target

    rev=6c9aa62eef53da4650bc34fa3e116ca34d875cfd
    work=$(mktemp -d)
    trap 'rm -rf -- "$work"' EXIT

    # cargo install 不继承当前 workspace 的 release profile。
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_LTO=true \
    CARGO_PROFILE_RELEASE_OPT_LEVEL=z CARGO_PROFILE_RELEASE_PANIC=abort \
    CARGO_PROFILE_RELEASE_STRIP=symbols \
        rustup run stable cargo install --git https://github.com/kongdd/edge-tts-rust \
            --rev "$rev" --target "$target" --root "$work" --bin edge-tts-rust edge-tts-rust
    curl -fsSL "https://raw.githubusercontent.com/kongdd/edge-tts-rust/$rev/LICENSE" \
        -o "$work/edge-tts-rust-LICENSE"

    release="target/$target/release"
    dist="dist/$platform"
    mkdir -p "$dist"
    cp "$release/clipboard-image$suffix" "$release/audio-player$suffix" \
        "$work/bin/edge-tts-rust$suffix" "$work/edge-tts-rust-LICENSE" "$dist/"
    if [[ "$platform" == windows-x64 ]]; then
        cp "$release/add-path.exe" "$dist/"
    fi
)

usage() {
    printf 'Usage: %s {windows-x64|macos-arm64}\n' "${0##*/}"
}

if [[ $# != 1 ]]; then
    usage >&2
    exit 2
fi

case "$1" in
    windows-x64)
        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
            build "$1" x86_64-pc-windows-gnu .exe
        ;;
    macos-arm64) build "$1" aarch64-apple-darwin ;;
    -h|--help) usage ;;
    *) usage >&2; exit 2 ;;
esac
