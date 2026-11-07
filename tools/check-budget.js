#!/usr/bin/env node
// M0 first-load budget. The brief's 8 MB M0 target is a cold first-load budget,
// so the gate is measured against Brotli bytes, which is what the network carries.
// Uncompressed bytes are reported for diagnosis but are not a transfer measure.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const zlib = require('node:zlib');

const BUDGETS = {
  firstLoadBrotliBytes: 8_000_000,
  wasmBrotliBytes: 12_000_000,
};

function evaluateBudget(artifacts) {
  const failures = [];
  const reports = {};
  for (const [renderer, artifact] of Object.entries(artifacts)) {
    const firstLoadBrotliBytes = artifact.wasmBrotliBytes + artifact.jsBytes
      + artifact.htmlBytes + artifact.loaderBytes + artifact.manifestBytes;
    reports[renderer] = { ...artifact, firstLoadBrotliBytes };
    if (firstLoadBrotliBytes >= BUDGETS.firstLoadBrotliBytes) {
      failures.push(`${renderer} exceeds the 8 MB Brotli first-load budget`);
    }
    if (artifact.wasmBrotliBytes > BUDGETS.wasmBrotliBytes) {
      failures.push(`${renderer} wasm exceeds the 12 MB Brotli budget`);
    }
  }
  return { failures, reports };
}

function measure(renderer, dist) {
  const dir = path.join(dist, renderer);
  const files = fs.readdirSync(dir);
  const js = files.filter(file => file.endsWith('.js'));
  const wasm = files.filter(file => file.endsWith('_bg.wasm'));
  if (js.length !== 1 || wasm.length !== 1) {
    throw new Error(`Unexpected Trunk outputs in ${dir}: ${files.join(', ')}`);
  }
  const data = fs.readFileSync(path.join(dir, wasm[0]));
  const brotli = zlib.brotliCompressSync(data, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11 } });
  const gzip = zlib.gzipSync(data, { level: 9 });
  fs.writeFileSync(path.join(dir, `${wasm[0]}.br`), brotli);
  fs.writeFileSync(path.join(dir, `${wasm[0]}.gz`), gzip);
  return {
    artifact: {
      js: `${renderer}/${js[0]}`,
      wasm: `${renderer}/${wasm[0]}`,
    },
    sizes: {
      wasmBytes: data.length,
      wasmBrotliBytes: brotli.length,
      wasmGzipBytes: gzip.length,
      jsBytes: fs.statSync(path.join(dir, js[0])).size,
      htmlBytes: fs.statSync(path.join(dist, 'index.html')).size,
      loaderBytes: fs.statSync(path.join(dist, 'loader.js')).size,
      manifestBytes: fs.statSync(path.join(dist, 'build-manifest.json')).size,
      sha256: crypto.createHash('sha256').update(data).digest('hex'),
    },
  };
}

function main() {
  const dist = process.argv[2] || 'dist';
  const manifestPath = path.join(dist, 'build-manifest.json');
  // A first build has no manifest yet, and the loader fetches it, so its bytes count.
  if (!fs.existsSync(manifestPath)) fs.writeFileSync(manifestPath, '{}\n');

  const manifest = {};
  const artifacts = {};
  for (const renderer of ['webgl2', 'webgpu']) {
    const { artifact, sizes } = measure(renderer, dist);
    manifest[renderer] = artifact;
    artifacts[renderer] = sizes;
  }
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  // The manifest and copied assets changed size or content, so measure them last.
  const manifestBytes = fs.statSync(manifestPath).size;
  const htmlBytes = fs.statSync(path.join(dist, 'index.html')).size;
  const loaderBytes = fs.statSync(path.join(dist, 'loader.js')).size;
  for (const sizes of Object.values(artifacts)) {
    Object.assign(sizes, { manifestBytes, htmlBytes, loaderBytes });
  }

  const { failures, reports } = evaluateBudget(artifacts);
  for (const [renderer, sizes] of Object.entries(reports)) {
    console.log(
      `${renderer}: wasm ${sizes.wasmBytes} bytes uncompressed, ${sizes.wasmBrotliBytes} Brotli, `
      + `${sizes.wasmGzipBytes} gzip; Brotli first load ${sizes.firstLoadBrotliBytes} bytes; `
      + `sha256 ${sizes.sha256}`,
    );
  }
  console.log(`budgets: Brotli first load < ${BUDGETS.firstLoadBrotliBytes} bytes, wasm Brotli <= ${BUDGETS.wasmBrotliBytes} bytes`);
  if (failures.length) {
    for (const failure of failures) console.error(failure);
    process.exitCode = 1;
  }
}

if (require.main === module) main();

module.exports = { BUDGETS, evaluateBudget, measure };
