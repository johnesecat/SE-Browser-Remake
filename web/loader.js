const status = document.querySelector('#status');
const progress = document.querySelector('#progress');
const loading = document.querySelector('#loading');
const retry = document.querySelector('#retry');
const diag = document.querySelector('#diag');
const params = new URLSearchParams(location.search);
retry.addEventListener('click', () => location.reload());

function fail(error) {
  console.error('Platform spike failed:', error);
  loading.hidden = false;
  status.textContent = `Unable to start: ${error.message || error}`;
  progress.hidden = true;
  retry.hidden = false;
}

async function probeWebGPU() {
  if (!navigator.gpu) return false;
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) return false;
  const device = await adapter.requestDevice();
  try {
    device.pushErrorScope('validation');
    const buffer = device.createBuffer({ size: 4, usage: GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(buffer, 0, new Uint32Array([42]));
    await device.queue.onSubmittedWorkDone();
    const error = await device.popErrorScope();
    buffer.destroy();
    return !error;
  } finally { device.destroy(); }
}

async function loadWasm(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Wasm download returned HTTP ${response.status}`);
  const length = Number(response.headers.get('content-length'));
  if (!response.body || !length || response.headers.get('content-encoding')) return response.arrayBuffer();
  const reader = response.body.getReader();
  const chunks = [];
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value); received += value.length;
    progress.value = Math.min(received / length, 1);
  }
  const data = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) { data.set(chunk, offset); offset += chunk.length; }
  return data;
}

async function start() {
  const override = params.get('renderer');
  if (override && !['webgpu', 'webgl2'].includes(override)) throw new Error('Unknown renderer override');
  let gpu = false;
  if (override !== 'webgl2') {
    try { gpu = await probeWebGPU(); } catch (error) { console.warn('WebGPU probe failed:', error); }
  }
  if (override === 'webgpu' && !gpu) throw new Error('WebGPU was requested but the adapter smoke test failed');
  const renderer = gpu ? 'webgpu' : 'webgl2';
  console.info(`Selected renderer artifact: ${renderer}`);
  document.documentElement.dataset.renderer = renderer.toUpperCase();
  document.documentElement.dataset.diagnostics = String(params.get('diag') === '1');
  diag.hidden = params.get('diag') !== '1';
  status.textContent = `Loading ${renderer.toUpperCase()} renderer…`;
  const manifest = await fetch('build-manifest.json', { cache: 'no-cache' });
  if (!manifest.ok) throw new Error(`Build manifest returned HTTP ${manifest.status}`);
  const artifacts = (await manifest.json())[renderer];
  const module = await import(new URL(artifacts.js, document.baseURI));
  const wasm = await loadWasm(new URL(artifacts.wasm, document.baseURI));
  status.textContent = 'Starting Bevy…';
  await module.default({ module_or_path: wasm });
  const deadline = performance.now() + 30000;
  while (document.documentElement.dataset.bevyReady !== 'true') {
    if (performance.now() > deadline) throw new Error('Bevy did not initialize within 30 seconds');
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  loading.hidden = true;
}
start().catch(fail);

let resizeTimer;
function resizeViewport() {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    if (window.visualViewport) document.querySelector('#stage').style.height = `${window.visualViewport.height}px`;
  }, 100);
}
window.visualViewport?.addEventListener('resize', resizeViewport);
resizeViewport();
