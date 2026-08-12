#!/usr/bin/env bash
# Build Osman.app and pack it into a drag-to-Applications DMG.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
APP_NAME=Osman
DIST=dist
STAGING="$DIST/dmg-staging"
DMG="$DIST/${APP_NAME}-${VERSION}.dmg"

echo "→ Building release app bundle"
"$(dirname "$0")/build-release.sh"

rm -rf "$STAGING"
mkdir -p "$STAGING"
cp -R "$DIST/$APP_NAME.app" "$STAGING/"
ln -sf /Applications "$STAGING/Applications"

rm -f "$DMG"
echo "→ Creating $DMG"
hdiutil create \
  -volname "Osman by NT" \
  -srcfolder "$STAGING" \
  -ov \
  -format UDZO \
  "$DMG" >/dev/null

rm -rf "$STAGING"

echo "→ $DMG ($(du -h "$DMG" | awk '{print $1}'))"
echo "   open \"$DMG\""
