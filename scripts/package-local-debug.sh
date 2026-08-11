#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'EOF'
Usage: ./scripts/package-local-debug.sh

Build a native macOS Debug application bundle for fast local UI verification.
The script reuses Cargo's incremental debug profile and never creates a DMG.

Output: target/debug/Tunnel Mate.app
EOF
  exit 0
fi

if [[ $# -ne 0 ]]; then
  echo "error: this script does not accept arguments" >&2
  echo "usage: ./scripts/package-local-debug.sh" >&2
  exit 2
fi

if ! command -v cargo-packager >/dev/null 2>&1; then
  echo "error: cargo-packager is not installed" >&2
  echo "install it with: cargo install cargo-packager --version 0.11.8 --locked" >&2
  exit 1
fi

profile="debug"
cargo build --locked -p tunnel-mate-gpui
cargo packager \
  --manifest-path apps/tunnel-mate-gpui/Cargo.toml \
  --formats app

echo "Application bundle: $repo_root/target/$profile/Tunnel Mate.app"
