#!/usr/bin/env bash
# One-time setup so a Mac can also build the Windows installer
# (`npm run build:all` then fills build/windows/). Tauri calls cross-building
# experimental: build/windows is produced, but test it on a Windows PC.
#
# Installs NSIS and LLVM (Homebrew), cargo-xwin, and the Rust Windows target.
# cargo-xwin downloads Microsoft's Windows SDK and C runtime on first use,
# which requires accepting Microsoft's license, so this script asks first.
set -euo pipefail

cd "$(dirname "$0")/.."
step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }
fail() { printf '\033[31merror:\033[0m %s\n' "$1" >&2; exit 1; }

[[ "$(uname)" == "Darwin" ]] || fail "This setup is for macOS. On Windows, npm run build:all builds the installer directly."
command -v brew >/dev/null || fail "Homebrew is missing. Install it from https://brew.sh"
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null || fail "Rust is missing. Install it from https://rustup.rs"

MARKER="$HOME/.sovirae-build/xwin-license-accepted"
if [[ ! -f "$MARKER" ]]; then
  cat <<'EOF'

Cross-building for Windows downloads the Microsoft Visual Studio C runtime and
Windows SDK (about 1 GB) through cargo-xwin. Using them means accepting
Microsoft's license terms:

  https://go.microsoft.com/fwlink/?LinkId=2086102

EOF
  read -r -p "Type yes to accept Microsoft's license and continue: " answer
  [[ "$answer" == "yes" ]] || fail "Not accepted. Nothing was installed."
fi

step "Installing NSIS and LLVM"
brew install nsis llvm

step "Adding the Rust Windows target"
rustup target add x86_64-pc-windows-msvc

step "Installing cargo-xwin"
cargo install --locked cargo-xwin

mkdir -p "$(dirname "$MARKER")"
date -u +"accepted %Y-%m-%dT%H:%M:%SZ" > "$MARKER"

step "Done"
echo "npm run build:all now builds macOS and Windows. Only Windows: npm run build:all -- --windows"
