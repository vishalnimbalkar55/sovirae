#!/usr/bin/env bash
# Builds the complete Sovirae project: checks prerequisites, installs
# JavaScript packages, runs every test, then bundles the macOS app and DMG
# (app + Kokoro worker + Chrome native host + Chrome extension), and
# gathers them into build/app and build/ext.
#
#   scripts/build.sh              full build with tests
#   scripts/build.sh --skip-tests build only
set -euo pipefail

cd "$(dirname "$0")/.."
SKIP_TESTS=0
[[ "${1:-}" == "--skip-tests" ]] && SKIP_TESTS=1

step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }
fail() { printf '\033[31merror:\033[0m %s\n' "$1" >&2; exit 1; }

step "Checking prerequisites"
[[ "$(uname)" == "Darwin" ]] || fail "The complete build currently targets macOS."
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null || fail "Rust is missing. Install it from https://rustup.rs"
command -v node >/dev/null || fail "Node.js is missing. Install Node.js 22 or later."
node_major=$(node -p 'process.versions.node.split(".")[0]')
(( node_major >= 22 )) || fail "Node.js 22 or later is required (found $(node --version))."
xcode-select -p >/dev/null 2>&1 || fail "Xcode Command Line Tools are missing. Run: xcode-select --install"
if ! command -v espeak-ng >/dev/null; then
  printf '\033[33mwarning:\033[0m espeak-ng is not installed. The app builds, but Kokoro voices need it:\n  brew install espeak-ng\n'
fi
echo "rust $(rustc --version | cut -d' ' -f2), node $(node --version), $(sw_vers -productName) $(sw_vers -productVersion)"

step "Installing JavaScript packages"
if [[ -f package-lock.json ]]; then npm ci --no-audit --no-fund; else npm install --no-audit --no-fund; fi

if (( ! SKIP_TESTS )); then
  step "Running tests"
  npm test
fi

step "Building the app, workers, and extension bundle"
npx tauri build

APP="target/release/bundle/macos/Sovirae.app"
DMG=$(ls target/release/bundle/dmg/*.dmg 2>/dev/null | head -1)
for f in speakit speakit-kokoro-worker speakit-native-host; do
  [[ -x "$APP/Contents/MacOS/$f" ]] || fail "$f is missing from the bundle."
done
# The Kokoro worker links WebGPU (Dawn) for GPU voices and cannot start without it.
[[ -f "$APP/Contents/Frameworks/libwebgpu_dawn.dylib" ]] || fail "libwebgpu_dawn.dylib is missing from the bundle."
[[ -f "$APP/Contents/Resources/ext/manifest.json" ]] || fail "The Chrome extension is missing from the bundle."

step "Collecting outputs into build/"
bash scripts/collect-build.sh
