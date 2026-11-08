// Renderer session core: which artifact to start, and when to give up.
//
// Everything here is a decision. The page supplies the effects — probing the
// GPU, fetching and importing the artifact, reading the running flag, moving to
// another URL, sleeping — so this module can be tested with plain fakes.
//
// Events are reported through onEvent:
//   { type: 'selected', renderer }  an artifact was chosen
//   { type: 'loading', renderer }   it is being fetched and started
//   { type: 'fallback', error }     WebGPU failed; the page is moving to WebGL2
//   { type: 'running' }             Bevy is updating; the scene may be shown
//   { type: 'failed', error }       the session is over and cannot continue

const RENDERERS = ['webgpu', 'webgl2'];

export function createSession({
  probe,
  startRenderer,
  isRunning,
  navigate,
  fallbackUrl,
  now,
  visibility,
  sleep,
  onEvent = () => {},
  timeoutMs = 30000,
  pollMs = 100,
}) {
  let renderer = null;
  let override = null;
  let fallback = null;
  let finished = false;
  let finalError = null;
  const emit = onEvent;

  const report = (type, detail = {}) => emit({ type, ...detail });

  function stop(error) {
    finished = true;
    finalError = error;
  }

  function fail(error) {
    if (finished) return false;
    stop(error);
    report('failed', { error });
    return true;
  }

  // A WebGPU session that was chosen automatically gets one reload with WebGL2.
  function handleFailure(error) {
    if (finished) return;
    if (renderer === 'webgpu' && override === null && fallback !== 'webgl2') {
      stop(error);
      report('fallback', { error });
      navigate(fallbackUrl());
      return;
    }
    fail(error);
  }

  // Waits for both the module promise and the Bevy update flag, counting only
  // time the page was visible: a backgrounded tab must not time out a slow load.
  async function waitForRunning(initialization) {
    let initialized = false;
    let initializationError = null;
    Promise.resolve(initialization).then(
      () => { initialized = true; },
      (error) => { initializationError = error || new Error('Initialization failed'); },
    );

    let visibleWait = 0;
    let lastCheck = now();
    let wasVisible = visibility();

    for (;;) {
      // A failure elsewhere (render error, lost context) ends the wait with its cause.
      if (finished) throw finalError ?? new Error('Session ended before the scene ran');
      if (initializationError) throw initializationError;
      if (initialized && isRunning()) return;

      const at = now();
      if (wasVisible) visibleWait += at - lastCheck;
      lastCheck = at;
      wasVisible = visibility();
      if (visibleWait > timeoutMs) {
        throw new Error('Bevy did not begin updating within 30 visible seconds');
      }
      await sleep(pollMs);
    }
  }

  async function start({ override: requested = null, fallback: used = null } = {}) {
    override = requested;
    fallback = used;

    if (override !== null && !RENDERERS.includes(override)) {
      throw Object.assign(new Error('Unknown renderer override'), { code: 'unknown-override' });
    }

    let gpu = false;
    const mayProbe = override !== 'webgl2' && (override === 'webgpu' || fallback !== 'webgl2');
    if (mayProbe) {
      try {
        gpu = await probe();
      } catch (error) {
        report('probe-error', { error });
      }
    }
    if (override === 'webgpu' && !gpu) {
      throw new Error('WebGPU was requested but the adapter smoke test failed');
    }

    renderer = gpu ? 'webgpu' : 'webgl2';
    report('selected', { renderer });
    report('loading', { renderer });

    let initialization;
    try {
      initialization = await startRenderer(renderer);
    } catch (error) {
      handleFailure(error);
      throw error;
    }

    await waitForRunning(initialization);
    if (finished) throw finalError ?? new Error('Session ended before the scene ran');
    report('running', { renderer });
  }

  return { start, fail: handleFailure, get renderer() { return renderer; } };
}
