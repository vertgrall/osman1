#!/usr/bin/env bash
# Regenerate docs/screenshots for README (About canvas renders + hero resize).
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p docs/screenshots

EXPORT_README=1 cargo test about_test_harness::tests::export_readme_screenshots -- --ignored --exact

sips -Z 720 resources/brand/SplashTowerVillage.png --out docs/screenshots/tower-village-hero.png >/dev/null
cp resources/brand/NewTowerBrandMark.png docs/screenshots/new-tower-brand-mark.png

echo "Wrote docs/screenshots/"
