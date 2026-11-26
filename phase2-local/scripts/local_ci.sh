#!/usr/bin/env bash
# Local CI: runs what the three GitHub workflows run, in order, stopping at the first failure.
#   fmt, clippy, workspace tests, scripts/server_checks.sh (server-checks.yml), the macOS
#   `cargo tauri build` and the Android release APK (alm-app-builds.yml), and a Windows
#   cross-build plus clippy for x86_64-pc-windows-msvc through cargo-xwin (alm-app-windows.yml
#   builds the NSIS installer on a Windows runner; here only the binary is cross-built).
# Prints one PASS/FAIL line per stage and the total time. Run from anywhere.
# Needs: Docker, the Tauri CLI 2.12.6, cargo-xwin, the aarch64-linux-android and
# x86_64-pc-windows-msvc targets, an Android SDK with NDK 28.2.13676358 and JDK 17.
# Environment: ANDROID_HOME, NDK_HOME and JAVA_HOME are found if unset.
set -euo pipefail

cd "$(dirname "$0")/.."
flags=(--locked --offline)

: "${ANDROID_HOME:=$(for d in "$HOME/Library/Android/sdk" /opt/homebrew/share/android-commandlinetools; do [ -d "$d" ] && echo "$d" && break; done)}"
: "${NDK_HOME:=$ANDROID_HOME/ndk/28.2.13676358}"
if [ -z "${JAVA_HOME:-}" ] && [ -d /opt/homebrew/opt/openjdk@17 ]; then
  JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
fi
export ANDROID_HOME NDK_HOME JAVA_HOME

start=$(date +%s)
export CARGO_FLAGS='--locked --offline'

stage() {
  local name=$1
  shift
  local t0=$(date +%s)
  echo "== $name"
  if "$@"; then
    local line="PASS $name ($(($(date +%s) - t0))s)"
    echo "$line"
  else
    local line="FAIL $name ($(($(date +%s) - t0))s)"
    echo "$line"
    echo "TOTAL $(($(date +%s) - start))s: FAIL"
    exit 1
  fi
}

fmt() { cargo fmt --all -- --check; }
clippy() { cargo clippy "${flags[@]}" --workspace --all-targets -- -D warnings; }
tests() (
  unset ALM_SKIP_PERF
  load=$(sysctl -n vm.loadavg | cut -d' ' -f2)
  if [[ -z ${CI:-} ]] && awk -v load="$load" 'BEGIN {exit !(load > 16)}'; then
    export ALM_SKIP_PERF=1
    echo "Local workspace timing bounds disabled: one-minute load $load > 16"
  fi
  cargo test "${flags[@]}" --workspace
)
server() { ./scripts/server_checks.sh; }
macos() { (cd crates/alm-app && cargo tauri build --bundles app,dmg -- "${flags[@]}"); }
android() { (cd crates/alm-app && cargo tauri android build --apk --target aarch64 -- "${flags[@]}"); }
windows() (
  # The NDK supplies the LLVM MSVC archiver and resource compiler needed by xwin.
  export PATH="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"
  cargo xwin build "${flags[@]}" --release -p alm-app --target x86_64-pc-windows-msvc &&
    cargo xwin clippy "${flags[@]}" --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings
)

stage fmt fmt
stage clippy clippy
stage tests tests
stage server-checks server
stage macos-app macos
stage android-apk android
stage windows-xwin windows

echo "TOTAL $(($(date +%s) - start))s: PASS"
