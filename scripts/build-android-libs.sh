#!/usr/bin/env bash
# Cross-compile tuner-core into Android .so files for every shipped ABI and copy
# them into the app's jniLibs.
#
# Requires:
#   - rustup with the four Android targets:
#       rustup target add aarch64-linux-android armv7-linux-androideabi \
#         x86_64-linux-android i686-linux-android
#   - the Android NDK (set ANDROID_NDK_HOME or NDK_HOME), r26+.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
core="$root/tuner-core"
jni_libs="$root/tuner-android/app/src/main/jniLibs"

ndk="${ANDROID_NDK_HOME:-${NDK_HOME:-}}"
if [[ -z "$ndk" ]]; then
  echo "error: set ANDROID_NDK_HOME (or NDK_HOME) to your NDK install" >&2
  exit 1
fi

host_tag="linux-x86_64" # change for macOS (darwin-x86_64) / Windows hosts
bin="$ndk/toolchains/llvm/prebuilt/$host_tag/bin"
api=26

# abi : triple : clang-wrapper
targets=(
  "arm64-v8a:aarch64-linux-android:aarch64-linux-android${api}-clang"
  "armeabi-v7a:armv7-linux-androideabi:armv7a-linux-androideabi${api}-clang"
  "x86_64:x86_64-linux-android:x86_64-linux-android${api}-clang"
  "x86:i686-linux-android:i686-linux-android${api}-clang"
)

for entry in "${targets[@]}"; do
  IFS=':' read -r abi triple clang <<<"$entry"
  linker="$bin/$clang"
  linker_var="CARGO_TARGET_$(echo "$triple" | tr '[:lower:]-' '[:upper:]_')_LINKER"

  echo ">> building $triple ($abi)"
  env "$linker_var=$linker" "CC_${triple}=$linker" \
    cargo build --manifest-path "$core/Cargo.toml" \
      --release --features jni --target "$triple"

  mkdir -p "$jni_libs/$abi"
  cp "$core/target/$triple/release/libtuner_core.so" "$jni_libs/$abi/libtuner_core.so"
done

echo "jniLibs populated under $jni_libs"
