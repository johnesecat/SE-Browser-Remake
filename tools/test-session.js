// Behaviour of the renderer session core, with no DOM and no browser.
//
// The core decides which artifact to use and when to give up; the page adapts
// those decisions to the DOM. That seam is what these tests stand on.
//
// No package.json exists in this repo, so Node detects the ES module syntax.
import { test } from "node:test";
import assert from "node:assert/strict";

import { createSession } from "../web/session.js";

// A session wired to plain fakes. Time only moves when a test advances it.
function harness(options = {}) {
  const {
    override = null,
    fallback = null,
    probeOk = true,
    probeThrows = false,
    rendererFails = false,
    visibility = () => true,
  } = options;

  let time = 0;
  let running = false;
  const events = [];
  const navigations = [];
  const probes = [];
  const started = [];
  const pending = [];

  const session = createSession({
    probe: async () => {
      probes.push(1);
      if (probeThrows) throw new Error("probe exploded");
      return probeOk;
    },
    startRenderer: async (renderer) => {
      started.push(renderer);
      if (rendererFails) throw new Error("wasm initialization failed");
      return "initialization";
    },
    isRunning: () => running,
    navigate: (url) => navigations.push(url),
    fallbackUrl: () => "https://example.test/game/?_fallback=webgl2",
    now: () => time,
    visibility,
    sleep: () => new Promise((resolve) => pending.push(resolve)),
    onEvent: (event) => events.push(event),
    timeoutMs: 300,
  });

  const settle = async () => {
    for (let i = 0; i < 20; i++) await Promise.resolve();
  };

  const advance = async (ms = 100) => {
    time += ms;
    for (const resolve of pending.splice(0)) resolve();
    await settle();
  };

  return {
    session,
    events,
    navigations,
    probes,
    started,
    types: () => events.map((event) => event.type),
    run() { running = true; },
    settle,
    advance,
    // Starts the session without awaiting it, so a test can move time forward.
    // Returns { outcome } so awaiting boot() does not also await the outcome.
    async boot() {
      const outcome = session.start({ override, fallback }).then(
        () => ({ ok: true }),
        (error) => ({ ok: false, message: error.message }),
      );
      await settle();
      return { outcome };
    },
  };
}

// Advances fake time until the start outcome settles, or gives up.
async function driveUntilSettled(app, outcome, message) {
  for (let i = 0; i < 30; i++) {
    const settled = await Promise.race([outcome, Promise.resolve(null)]);
    if (settled) return settled;
    await app.advance();
  }
  assert.fail(message);
}

test("selects the WebGL2 artifact when no GPU probe is available", async () => {
  const app = harness({ probeOk: false });
  const { outcome } = await app.boot();
  assert.equal(await Promise.race([outcome, Promise.resolve(null)]), null,
    "startup stays pending while the scene is not updating");
  assert.equal(app.probes.length, 1);
  assert.deepEqual(app.started, ["webgl2"]);
  assert.equal(app.events[0].type, "selected");
  assert.equal(app.events[0].renderer, "webgl2");
  assert.ok(app.types().includes("loading"), "the loading state is announced");
});

test("resolves and reports running once Bevy is updating", async () => {
  const app = harness({ probeOk: false });
  const { outcome } = await app.boot();
  app.run();
  assert.deepEqual(await driveUntilSettled(app, outcome, "a running scene never resolved"), { ok: true });
  assert.equal(app.types().at(-1), "running");
});

test("surfaces a wasm initialization failure instead of waiting forever", async () => {
  const app = harness({ probeOk: false, rendererFails: true });
  const { outcome } = await app.boot();
  const result = await driveUntilSettled(app, outcome, "a failed initialization never settled");
  assert.equal(result.ok, false);
  assert.match(result.message, /wasm initialization failed/);
  assert.ok(app.types().includes("failed"));
});

test("counts only visible time toward the startup timeout", async () => {
  let visible = false;
  const app = harness({ probeOk: false, visibility: () => visible });
  const { outcome } = await app.boot();

  await app.advance(100);
  await app.advance(100);
  await app.advance(100);
  await app.advance(100);
  assert.equal(await Promise.race([outcome, Promise.resolve(null)]), null,
    "hidden time must not expire the watchdog");

  visible = true;
  const result = await driveUntilSettled(app, outcome, "visible time never expired the watchdog");
  assert.equal(result.ok, false);
  assert.match(result.message, /did not begin updating/);
});

test("falls back to WebGL2 once when WebGPU fails on its own", async () => {
  const app = harness({ probeOk: true });
  await app.boot();
  assert.deepEqual(app.started, ["webgpu"]);

  app.session.fail(new Error("device lost"));
  await app.settle();
  assert.ok(app.types().includes("fallback"), "a fallback is announced");
  assert.equal(app.navigations.length, 1);
  assert.match(app.navigations[0], /_fallback=webgl2/);
  assert.ok(!app.types().includes("failed"), "a fallback is not a final failure");
});

test("does not fall back a second time", async () => {
  const app = harness({ probeOk: true });
  await app.boot();
  app.session.fail(new Error("device lost"));
  app.session.fail(new Error("device lost again"));
  await app.settle();
  assert.equal(app.navigations.length, 1);
});

test("a forced WebGPU failure is final instead of falling back", async () => {
  const app = harness({ override: "webgpu", probeOk: true });
  await app.boot();
  app.session.fail(new Error("device lost"));
  await app.settle();
  assert.equal(app.navigations.length, 0);
  assert.ok(app.types().includes("failed"));
});

test("a forced WebGPU request fails when the probe cannot get a device", async () => {
  const app = harness({ override: "webgpu", probeOk: false });
  const { outcome } = await app.boot();
  const result = await outcome;
  assert.equal(result.ok, false);
  assert.match(result.message, /adapter smoke test failed/);
  assert.deepEqual(app.started, [], "no artifact is loaded without a device");
});

test("a fallback navigation skips the WebGPU probe", async () => {
  const app = harness({ fallback: "webgl2", probeOk: true });
  await app.boot();
  assert.deepEqual(app.probes, [], "the probe is not repeated after a fallback");
  assert.deepEqual(app.started, ["webgl2"]);
});

test("an unknown override fails before probing or loading anything", async () => {
  const app = harness({ override: "bogus" });
  const { outcome } = await app.boot();
  const result = await outcome;
  assert.equal(result.ok, false);
  assert.match(result.message, /Unknown renderer override/);
  assert.deepEqual(app.probes, []);
  assert.deepEqual(app.started, []);
});

test("a probe that throws is treated as no WebGPU, not as a crash", async () => {
  const app = harness({ probeThrows: true });
  await app.boot();
  assert.deepEqual(app.started, ["webgl2"]);
});

test("a render error ends a startup that has not finished", async () => {
  const app = harness({ probeOk: false });
  const { outcome } = await app.boot();
  app.session.fail(new Error("Renderer failed: device lost"));
  const result = await driveUntilSettled(app, outcome, "a render error never settled the start");
  assert.equal(result.ok, false);
  assert.match(result.message, /device lost/);
  assert.equal(app.types().filter((type) => type === "failed").length, 1,
    "a single failure is reported once");
});
