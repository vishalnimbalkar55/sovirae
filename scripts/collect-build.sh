#!/usr/bin/env bash
# Gathers the build outputs into one folder:
#
#   build/app/Sovirae.app          the app (workers and native host inside)
#   build/app/Sovirae_*.dmg        the installer
#   build/ext/                     the Chrome extension, ready for Load unpacked
#
# Runs after `tauri build` (npm run app:build and npm run build:all).
set -euo pipefail

cd "$(dirname "$0")/.."
fail() { printf '\033[31merror:\033[0m %s\n' "$1" >&2; exit 1; }

BUNDLE="target/release/bundle"
APP="$BUNDLE/macos/Sovirae.app"
[[ -d "$APP" ]] || fail "$APP is missing. Run npm run app:build first."

rm -rf build/app build/ext
mkdir -p build/app build/ext

# ditto keeps the bundle's symlinks, permissions, and signature intact.
ditto "$APP" build/app/Sovirae.app
for dmg in "$BUNDLE"/dmg/*.dmg; do
  [[ -f "$dmg" ]] && cp "$dmg" build/app/
done

# The extension without its tests and dev-only package file.
rsync -a --exclude tests --exclude package.json --exclude node_modules --exclude .DS_Store ext/ build/ext/
[[ -f build/ext/manifest.json ]] || fail "The Chrome extension was not copied."

echo "App:       build/app/Sovirae.app"
echo "Installer: $(ls build/app/*.dmg 2>/dev/null | head -1 || echo 'not built')"
echo "Extension: build/ext"
