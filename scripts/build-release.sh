#!/usr/bin/env bash
# Build release binary and bundle Osman.app with clinical-scope icon.
set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME=Osman
BUNDLE_ID=com.newtower.osman
DIST=dist
APP="$DIST/$APP_NAME.app"
BINARY=target/release/osman1

echo "→ cargo build --release"
cargo build --release

if [[ ! -f resources/icon/AppIcon.icns ]]; then
  echo "Missing AppIcon.icns — build.rs should have created it" >&2
  exit 1
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/$APP_NAME"
cp resources/Info.plist "$APP/Contents/Info.plist"
cp resources/icon/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$APP/Contents/Info.plist" 2>/dev/null || true

chmod +x "$APP/Contents/MacOS/$APP_NAME"
touch "$APP"

echo "→ $APP"
echo "   open \"$APP\""
