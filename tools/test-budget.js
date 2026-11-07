const assert = require('node:assert/strict');
const { evaluateBudget } = require('./check-budget.js');

// Real measured artifact sizes from the 2026-11-07 optimized builds.
const webgl2 = {
  wasmBytes: 13_532_974, wasmBrotliBytes: 2_784_752,
  jsBytes: 103_462, htmlBytes: 2_100, loaderBytes: 4_800, manifestBytes: 200,
};
const webgpu = {
  wasmBytes: 11_944_691, wasmBrotliBytes: 2_309_505,
  jsBytes: 110_587, htmlBytes: 2_100, loaderBytes: 4_800, manifestBytes: 200,
};

const firstLoad = a => a.wasmBrotliBytes + a.jsBytes + a.htmlBytes + a.loaderBytes + a.manifestBytes;

// Measured builds must satisfy the M0 transfer budget, not the uncompressed byte count.
{
  const result = evaluateBudget({ webgl2, webgpu });
  assert.deepEqual(result.failures, []);
  assert.equal(firstLoad(webgl2), 2_895_314);
  assert.equal(result.reports.webgl2.firstLoadBrotliBytes, 2_895_314);
  assert.equal(result.reports.webgl2.wasmBytes, 13_532_974, 'uncompressed bytes stay reported');
}

// The gate is an 8 MB transfer budget, so exactly 8 MB must fail.
{
  const brotli = 8_000_000 - (webgl2.jsBytes + webgl2.htmlBytes + webgl2.loaderBytes + webgl2.manifestBytes);
  const result = evaluateBudget({ webgl2: { ...webgl2, wasmBrotliBytes: brotli } });
  assert.deepEqual(result.failures, ['webgl2 exceeds the 8 MB Brotli first-load budget']);
}

// A wasm over the 12 MB Brotli ceiling reports both the wasm and total-load failures.
{
  const result = evaluateBudget({ webgpu: { ...webgpu, wasmBrotliBytes: 12_000_001 } });
  assert.deepEqual(result.failures, [
    'webgpu exceeds the 8 MB Brotli first-load budget',
    'webgpu wasm exceeds the 12 MB Brotli budget',
  ]);
}

// Both renderers are reported, and a healthy renderer does not mask a failing one.
{
  const result = evaluateBudget({ webgl2, webgpu: { ...webgpu, wasmBrotliBytes: 12_500_000 } });
  assert.deepEqual(result.failures, [
    'webgpu exceeds the 8 MB Brotli first-load budget',
    'webgpu wasm exceeds the 12 MB Brotli budget',
  ]);
  assert.equal(result.reports.webgpu.wasmBrotliBytes, 12_500_000);
  assert.equal(result.reports.webgl2.wasmBrotliBytes, 2_784_752);
}

console.log('Budget gate checks passed.');
