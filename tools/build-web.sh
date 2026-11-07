#!/usr/bin/env bash
set -euo pipefail

# The repository's project-pages URL, or / for local hosting.
public_url="${1:-/SE-Browser-Remake/}"
case "$public_url" in /|/*/) ;; *) echo 'public URL must start and end with /' >&2; exit 1;; esac

if [ "$(trunk --version)" != 'trunk 0.22.1' ]; then
  echo 'Install Trunk 0.22.1 before building.' >&2
  exit 1
fi

mkdir -p dist
for renderer in webgl2 webgpu; do
  trunk build --release --locked --features "$renderer" --dist "dist/$renderer" --public-url "$public_url$renderer/"
done
cp web/index.html web/loader.js dist/
touch dist/.nojekyll

# Gate the M0 first-load transfer budget (Brotli) and report uncompressed sizes.
node tools/check-budget.js dist
