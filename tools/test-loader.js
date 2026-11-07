const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

function loader(search = '') {
  const timers = [];
  let now = 0;
  const elements = Object.fromEntries(['status', 'progress', 'loading', 'retry', 'diag', 'game'].map(id => [id, {
    hidden: false, textContent: '', addEventListener() {}, style: {},
  }]));
  const listeners = new Map();
  const document = {
    visibilityState: 'visible', documentElement: { dataset: {} },
    querySelector: selector => elements[selector.slice(1)],
    addEventListener: (name, callback) => listeners.set(name, callback),
    removeEventListener: (name, callback) => { if (listeners.get(name) === callback) listeners.delete(name); },
  };
  const location = { search, href: `https://example.test/game/${search}`, replace(url) { this.replaced = url; }, reload() {} };
  const context = vm.createContext({
    document, location, navigator: {}, URL, URLSearchParams,
    console: { error() {}, info() {}, warn() {} },
    window: {}, MutationObserver: class { observe() {} },
    performance: { now: () => now },
    setTimeout: callback => { timers.push(callback); return timers.length; }, clearTimeout() {},
  });
  const source = fs.readFileSync('web/loader.js', 'utf8')
    .replace('start().catch(handleFailure);', '')
    .concat('\nglobalThis.api = { waitForBevy, handleFailure, setRenderer(value) { renderer = value; } };');
  vm.runInContext(source, context, { filename: 'web/loader.js' });
  return {
    api: context.api, document, location, elements,
    setVisibility(value) { document.visibilityState = value; listeners.get('visibilitychange')?.(); },
    async tick(ms = 100) {
      now += ms;
      const pending = timers.splice(0);
      for (const callback of pending) callback();
      await Promise.resolve();
      await Promise.resolve();
    },
  };
}

async function advanceUntilSettled(app, promise, message) {
  let settled = false;
  promise.then(() => { settled = true; }, () => { settled = true; });
  for (let i = 0; i < 20 && !settled; i++) await app.tick();
  assert.equal(settled, true, message);
  return promise;
}

async function main() {
  {
    const app = loader();
    const outcome = app.api.waitForBevy(new Promise(() => {}), 300).then(() => 'resolved', error => error.message);
    assert.match(await advanceUntilSettled(app, outcome, 'hung initialization did not time out'), /did not begin updating/);
  }
  {
    const app = loader();
    let settled = false;
    const outcome = app.api.waitForBevy(new Promise(() => {}), 300).then(() => { settled = true; }, () => { settled = true; });
    app.setVisibility('hidden');
    for (let i = 0; i < 10; i++) await app.tick(1000);
    assert.equal(settled, false, 'hidden time must not exhaust the timeout');
    app.setVisibility('visible');
    await advanceUntilSettled(app, outcome, 'visible timeout did not complete');
    assert.equal(settled, true);
  }
  {
    const app = loader();
    const outcome = app.api.waitForBevy(new Promise(() => {}), 300).then(() => 'resolved', error => error.message);
    app.setVisibility('hidden');
    app.setVisibility('visible');
    await app.tick(10000);
    assert.match(await advanceUntilSettled(app, outcome, 'visible time after backgrounding was ignored'), /did not begin updating/);
  }
  {
    const app = loader();
    const outcome = app.api.waitForBevy(Promise.reject(new Error('init failed'))).then(() => 'resolved', error => error.message);
    assert.equal(await advanceUntilSettled(app, outcome, 'initializer rejection did not propagate'), 'init failed');
  }
  {
    const app = loader();
    app.document.documentElement.dataset.bevyRunning = 'true';
    const outcome = app.api.waitForBevy(Promise.resolve());
    assert.equal(await advanceUntilSettled(app, outcome, 'successful initialization did not complete'), true);
  }
  {
    const app = loader();
    const outcome = app.api.waitForBevy(new Promise(() => {}));
    app.api.handleFailure(new Error('device lost'));
    assert.equal(await advanceUntilSettled(app, outcome, 'render failure did not stop watchdog'), false);
    assert.match(app.elements.status.textContent, /device lost/);
  }
  {
    const app = loader('?diag=1');
    app.api.setRenderer('webgpu');
    app.api.handleFailure(new Error('device lost'));
    assert.equal(new URL(app.location.replaced).searchParams.get('_fallback'), 'webgl2');
    const first = app.location.replaced;
    app.api.handleFailure(new Error('again'));
    assert.equal(app.location.replaced, first);
  }
  {
    const app = loader('?renderer=webgpu');
    app.api.setRenderer('webgpu');
    app.api.handleFailure(new Error('device lost'));
    assert.equal(app.location.replaced, undefined);
    assert.match(app.elements.status.textContent, /device lost/);
  }
  console.log('Loader watchdog and fallback checks passed.');
}

main().catch(error => { console.error(error); process.exitCode = 1; });
