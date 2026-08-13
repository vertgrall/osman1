#!/usr/bin/env bash
# Render docs/Osman-User-Guide.html to a PDF on the Desktop.
set -euo pipefail
cd "$(dirname "$0")/.."

HTML="$(pwd)/docs/Osman-User-Guide.html"
OUT="${1:-$HOME/Desktop/Osman-User-Guide.pdf}"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

if [[ ! -f "$HTML" ]]; then
  echo "Missing $HTML" >&2
  exit 1
fi

if [[ ! -x "$CHROME" ]]; then
  echo "Google Chrome not found — install Chrome or pass output path after installing pandoc." >&2
  exit 1
fi

echo "→ PDF: $OUT"
"$CHROME" \
  --headless=new \
  --disable-gpu \
  --no-pdf-header-footer \
  --print-to-pdf="$OUT" \
  "file://$HTML"

echo "→ Done ($(du -h "$OUT" | awk '{print $1}'))"
echo "   open \"$OUT\""
