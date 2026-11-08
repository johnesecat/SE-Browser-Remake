// DOM adapter for the renderer session: fetches, imports and reports.
// The decisions about which artifact to start and when to give up live in
// ./session.js so they can be tested without a browser.
import { createSession } from './session.js';

const root = document.documentElement;
const status = document.querySelector('#status');
const progress = document.querySelector('#progress');
const loading = document.querySelector('#loading');
const retry = document.querySelector('#retry');
const diag = document.querySelector('#diag');
const canvas = document.querySelector('#game');
const params = new URLSearchParams(location.search);

retry.addEventListener('click', () => location.reload());

// The session reports its own failures; this only covers errors it never saw.
let handled = false;

function showFailure(error) {
  handled = true;
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

const session = createSession({
  probe: probeWebGPU,
  startRenderer: async (renderer) => {
    const manifest = await fetch('build-manifest.json', { cache: 'no-cache' });
    if (!manifest.ok) throw new Error(`Build manifest returned HTTP ${manifest.status}`);
    const artifacts = (await manifest.json())[renderer];
    const module = await import(new URL(artifacts.js, document.baseURI));
    const wasm = await loadWasm(new URL(artifacts.wasm, document.baseURI));
    status.textContent = 'Starting Bevy…';
    // The watchdog covers wasm initialization as well as the Bevy update loop.
    return module.default({ module_or_path: wasm });
  },
  isRunning: () => root.dataset.bevyRunning === 'true',
  navigate: (url) => location.replace(url),
  fallbackUrl: () => {
    const url = new URL(location.href);
    url.searchParams.set('_fallback', 'webgl2');
    return url.href;
  },
  now: () => performance.now(),
  visibility: () => document.visibilityState === 'visible',
  sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
  onEvent: (event) => {
    switch (event.type) {
      case 'selected':
        console.info(`Selected renderer artifact: ${event.renderer}`);
        root.dataset.renderer = event.renderer.toUpperCase();
        root.dataset.diagnostics = String(params.get('diag') === '1');
        diag.hidden = params.get('diag') !== '1';
        break;
      case 'loading':
        status.textContent = `Loading ${event.renderer.toUpperCase()} renderer…`;
        break;
      case 'fallback':
        handled = true;
        console.error('WebGPU failed; retrying with WebGL2:', event.error);
        status.textContent = 'WebGPU failed; retrying with WebGL2…';
        break;
      case 'probe-error':
        console.warn('WebGPU probe failed:', event.error);
        break;
      case 'running':
        loading.hidden = true;
        break;
      case 'failed':
        showFailure(event.error);
        break;
      default:
        break;
    }
  },
});

// A renderer that reports a fatal error after startup takes the same path.
new MutationObserver(() => {
  if (root.dataset.renderError) session.fail(new Error(`Renderer failed: ${root.dataset.renderError}`));
}).observe(root, { attributes: true, attributeFilter: ['data-render-error'] });
canvas.addEventListener('webglcontextlost', () => {
  session.fail(new Error('WebGL context lost; reload to retry'));
});

session
  .start({ override: params.get('renderer'), fallback: params.get('_fallback') })
  .catch((error) => { if (!handled) showFailure(error); });

let resizeTimer;
function resizeViewport() {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    if (window.visualViewport) document.querySelector('#stage').style.height = `${window.visualViewport.height}px`;
  }, 100);
}
window.visualViewport?.addEventListener('resize', resizeViewport);
resizeViewport();
