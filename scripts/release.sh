#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

usage() {
  cat <<'EOF'
Usage: ./scripts/release.sh <version> [--yes]

Prepare and publish a tagged release. The version must omit the leading "v".
The script updates Cargo.toml and Cargo.lock, commits the version bump, then asks
before pushing main and the version tag. All checks and packaging run on GitHub.

Options:
  -y, --yes  Skip the final publish confirmation.
  -h, --help Show this help.

Example:
  ./scripts/release.sh 0.5.2
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

version="${1:-}"
assume_yes=false

if [[ "${2:-}" == "--yes" || "${2:-}" == "-y" ]]; then
  assume_yes=true
elif [[ -n "${2:-}" ]]; then
  echo "error: unknown option: $2" >&2
  usage >&2
  exit 2
fi

if [[ -z "$version" || $# -gt 2 ]]; then
  usage >&2
  exit 2
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: invalid version '$version' (use a value such as 0.5.2)" >&2
  exit 2
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: the working tree is not clean" >&2
  echo "commit or stash the current changes before preparing a release" >&2
  exit 1
fi

branch="$(git branch --show-current)"
if [[ "$branch" != "main" ]]; then
  echo "error: releases must be prepared from main (current branch: $branch)" >&2
  exit 1
fi

tag="v$version"
if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
  echo "error: local tag $tag already exists" >&2
  exit 1
fi

current_version="$(awk '
  /^\[workspace\.package\]$/ { in_workspace_package = 1; next }
  /^\[/ { in_workspace_package = 0 }
  in_workspace_package && /^version[[:space:]]*=/ {
    value = $0
    sub(/^[^"]*"/, "", value)
    sub(/".*/, "", value)
    print value
    exit
  }
' Cargo.toml)"

if [[ -z "$current_version" ]]; then
  echo "error: could not read workspace.package.version from Cargo.toml" >&2
  exit 1
fi

if [[ "$current_version" == "$version" ]]; then
  echo "error: Cargo.toml is already at version $version" >&2
  exit 1
fi

version_file="$(mktemp "${TMPDIR:-/tmp}/tunnel-mate-version.XXXXXX")"
cleanup() {
  rm -f "$version_file"
}
trap cleanup EXIT

awk -v new_version="$version" '
  /^\[workspace\.package\]$/ { in_workspace_package = 1 }
  /^\[/ && $0 != "[workspace.package]" { in_workspace_package = 0 }
  in_workspace_package && /^version[[:space:]]*=/ && !updated {
    print "version = \"" new_version "\""
    updated = 1
    next
  }
  { print }
  END {
    if (!updated) exit 1
  }
' Cargo.toml > "$version_file"
mv "$version_file" Cargo.toml

# Refresh the workspace package versions in Cargo.lock without compiling,
# testing, packaging, or accessing the network.
cargo metadata --format-version 1 --offline >/dev/null

git add Cargo.toml Cargo.lock
git commit -m "release: $tag"

if [[ "$assume_yes" != true ]]; then
  printf "Publish %s by pushing main and the new tag? [y/N] " "$tag"
  read -r answer
  if [[ ! "$answer" =~ ^[Yy]$ ]]; then
    echo "Release commit created locally; nothing was pushed and no tag was created."
    echo "Run ./scripts/release.sh again only after resetting the local release commit,"
    echo "or publish it manually when ready."
    exit 0
  fi
fi

git push origin main
git tag -a "$tag" -m "Tunnel Mate $tag"
git push origin "$tag"

echo "Published $tag"
