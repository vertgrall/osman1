#!/usr/bin/env bash
# Generate macOS app icon sizes from the clinical-scope master PNG.
set -euo pipefail
cd "$(dirname "$0")/.."

MASTER=resources/icon/OsmanAppIcon-1024.png
OUT=resources/icon/AppIcon.appiconset

if [[ ! -f "$MASTER" ]]; then
  echo "Missing master icon: $MASTER" >&2
  echo "Copy clinical mock: cp resources/brand/osman-icon-mock-04-clinical-scope.png $MASTER" >&2
  exit 1
fi

mkdir -p "$OUT"
sips -z 16 16 "$MASTER" --out "$OUT/Icon-16.png" >/dev/null
sips -z 32 32 "$MASTER" --out "$OUT/Icon-32.png" >/dev/null
sips -z 64 64 "$MASTER" --out "$OUT/Icon-64.png" >/dev/null
sips -z 128 128 "$MASTER" --out "$OUT/Icon-128.png" >/dev/null
sips -z 256 256 "$MASTER" --out "$OUT/Icon-256.png" >/dev/null
sips -z 512 512 "$MASTER" --out "$OUT/Icon-512.png" >/dev/null
cp "$MASTER" "$OUT/Icon-1024.png"
sips -z 22 22 "$MASTER" --out resources/icon/MenubarIcon-22.png >/dev/null
sips -z 128 128 "$MASTER" --out resources/icon/WindowIcon-128.png >/dev/null

echo "Icon PNGs updated. AppIcon.icns is built by build.rs on next cargo build."
