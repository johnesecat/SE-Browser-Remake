const status = document.querySelector('#status');
const progress = document.querySelector('#progress');
const loading = document.querySelector('#loading');
const retry = document.querySelector('#retry');
const diag = document.querySelector('#diag');
const params = new URLSearchParams(location.search);
retry.addEventListener('click', () => location.reload());

let renderer;
let failed = false;

function handleFailure(error) {
  if (failed) return;
  failed = true;
  if (renderer === 'webgpu' && !params.has('renderer') && params.get('_fallback') !== 'webgl2') {
    console.error('WebGPU failed; retrying with WebGL2:', error);
    status.textContent = 'WebGPU failed; retrying with WebGL2…';
    const url = new URL(location.href);
    url.searchParams.set('_fallback', 'webgl2');
    location.replace(url.href);
    return;
  }
  fail(error);
}

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

async function waitForBevy(initialization, timeoutMs = 30000) {
  let initialized = false;
  let initializationFailed = false;
  let initializationError;
  Promise.resolve(initialization).then(
    () => { initialized = true; },
    error => { initializationFailed = true; initializationError = error; },
  );
  let visibleWait = 0;
  let lastCheck = performance.now();
  let wasVisible = document.visibilityState === 'visible';
  function updateVisibility() {
    const now = performance.now();
    if (wasVisible) visibleWait += now - lastCheck;
    lastCheck = now;
    wasVisible = document.visibilityState === 'visible';
  }
  document.addEventListener('visibilitychange', updateVisibility);
  try {
    while (!initialized || document.documentElement.dataset.bevyRunning !== 'true') {
      if (failed) return false;
      if (initializationFailed) throw initializationError;
      updateVisibility();
      if (visibleWait > timeoutMs) throw new Error('Bevy did not begin updating within 30 visible seconds');
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    return !failed;
  } finally {
    document.removeEventListener('visibilitychange', updateVisibility);
  }
}

async function start() {
  const override = params.get('renderer');
  if (override !== null && !['webgpu', 'webgl2'].includes(override)) throw new Error('Unknown renderer override');
  let gpu = false;
  if (override !== 'webgl2' && (override === 'webgpu' || params.get('_fallback') !== 'webgl2')) {
    try { gpu = await probeWebGPU(); } catch (error) { console.warn('WebGPU probe failed:', error); }
  }
  if (override === 'webgpu' && !gpu) throw new Error('WebGPU was requested but the adapter smoke test failed');
  renderer = gpu ? 'webgpu' : 'webgl2';
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
  // The watchdog covers wasm initialization as well as the Bevy update loop.
  // A progressing app loop is not proof of a successful visible frame.
  if (await waitForBevy(module.default({ module_or_path: wasm }))) loading.hidden = true;
}

const root = document.documentElement;
new MutationObserver(() => {
  if (root.dataset.renderError) handleFailure(new Error(`Renderer failed: ${root.dataset.renderError}`));
}).observe(root, { attributes: true, attributeFilter: ['data-render-error'] });
document.querySelector('#game').addEventListener('webglcontextlost', () => {
  handleFailure(new Error('WebGL context lost; reload to retry'));
});
start().catch(handleFailure);

let resizeTimer;
function resizeViewport() {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    if (window.visualViewport) document.querySelector('#stage').style.height = `${window.visualViewport.height}px`;
  }, 100);
}
window.visualViewport?.addEventListener('resize', resizeViewport);
resizeViewport();
