#!/usr/bin/env bash
# Assemble the deployable Pages site from the build output.
#
# The manifest is the source of truth: only the entry document, the loader and
# the files it names are copied, so stale experiments in dist/ (webgl2-new,
# webgpu-new, webgpu-cpu) and the .br/.gz sidecars cannot reach the deploy.
#
# Usage: package-site.sh [DIST_DIR] [OUT_DIR]
set -euo pipefail

dist=${1:-dist}
out=${2:-_site}

[ -f "$dist/build-manifest.json" ] || {
  echo "package-site: $dist/build-manifest.json is missing; run tools/build-web.sh first" >&2
  exit 1
}
for required in index.html loader.js; do
  [ -f "$dist/$required" ] || { echo "package-site: $dist/$required is missing" >&2; exit 1; }
done

# A stale file from an earlier package must not survive into the deploy.
rm -rf "$out"
mkdir -p "$out"

copy() {
  local relative=$1
  [ -f "$dist/$relative" ] || { echo "package-site: $dist/$relative is missing" >&2; exit 1; }
  mkdir -p "$out/$(dirname "$relative")"
  cp "$dist/$relative" "$out/$relative"
}

copy index.html
copy loader.js
copy build-manifest.json

# Renderer entry points and the artifacts the loader injects for each backend.
while IFS= read -r relative; do
  case "$relative" in
    *.br | *.gz | *.map)
      echo "package-site: refusing to ship $relative" >&2
      exit 1
      ;;
  esac
  copy "$relative"
done < <(node - "$dist/build-manifest.json" <<'NODE'
const fs = require("node:fs");
const manifest = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
for (const renderer of Object.keys(manifest)) {
  const entry = manifest[renderer];
  for (const key of ["js", "wasm"]) {
    if (!entry[key]) throw new Error(`${renderer} has no ${key} in the manifest`);
    console.log(entry[key]);
  }
  console.log(`${entry.js.replace(/\/[^/]+$/, "")}/index.html`);
}
NODE
)

echo "packaged $(find "$out" -type f | wc -l | tr -d ' ') files into $out"
