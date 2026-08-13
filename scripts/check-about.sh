#!/usr/bin/env bash
# About branding regression gate — run before merging UI/menu changes.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "→ about_contract (source + pixel guards)"
cargo test about_contract --quiet

echo "→ about UI + asset tests"
cargo test about_ --quiet

echo "✓ About regression checks passed"
