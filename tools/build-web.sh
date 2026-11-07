#!/usr/bin/env bash
set -euo pipefail

# The repository's project-pages URL, or / for local hosting.
public_url="${1:-/SE-Browser-Remake/}"
case "$public_url" in /*/) ;; *) echo 'public URL must start and end with /' >&2; exit 1;; esac

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

node <<'NODE'
const fs = require('node:fs');
const crypto = require('node:crypto');
const zlib = require('node:zlib');
const manifest = {};
for (const renderer of ['webgl2', 'webgpu']) {
  const dir = `dist/${renderer}`;
  const files = fs.readdirSync(dir);
  const js = files.filter(f => f.endsWith('.js'));
  const wasm = files.filter(f => f.endsWith('_bg.wasm'));
  if (js.length !== 1 || wasm.length !== 1) throw new Error(`Unexpected Trunk outputs in ${dir}: ${files}`);
  manifest[renderer] = { js: `${renderer}/${js[0]}`, wasm: `${renderer}/${wasm[0]}` };
  const data = fs.readFileSync(`${dir}/${wasm[0]}`);
  const br = zlib.brotliCompressSync(data, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11 } });
  const gz = zlib.gzipSync(data, { level: 9 });
  fs.writeFileSync(`${dir}/${wasm[0]}.br`, br);
  fs.writeFileSync(`${dir}/${wasm[0]}.gz`, gz);
  console.log(`${renderer}: wasm ${data.length} bytes, brotli ${br.length} bytes, gzip ${gz.length} bytes, sha256 ${crypto.createHash('sha256').update(data).digest('hex')}`);
  if (br.length > 12_000_000) throw new Error(`${renderer} exceeds 12 MB Brotli`);
  const firstLoad = data.length + fs.statSync(`${dir}/${js[0]}`).size + fs.statSync('dist/index.html').size + fs.statSync('dist/loader.js').size;
  console.log(`${renderer}: conservative uncompressed initial HTML + JS + wasm: ${firstLoad} bytes (does not include response headers/manifest)`);
  if (firstLoad >= 8_000_000) throw new Error(`${renderer} exceeds M0's 8 MB conservative initial-load gate`);
}
fs.writeFileSync('dist/build-manifest.json', JSON.stringify(manifest, null, 2) + '\n');
NODE
