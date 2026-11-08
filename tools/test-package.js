#!/usr/bin/env node
// Checks that the Pages package contains exactly the files the loader needs.
const { test } = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const repo = path.join(__dirname, "..");
const script = path.join(repo, "tools", "package-site.sh");
const dist = path.join(repo, "dist");

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(full) : [full];
  });
}

function packageInto(target) {
  execFileSync("bash", [script, dist, target], { stdio: "pipe" });
  return walk(target)
    .map((file) => path.relative(target, file).split(path.sep).join("/"))
    .sort();
}

test("packages a site into a clean output directory", () => {
  const target = fs.mkdtempSync(path.join(os.tmpdir(), "site-"));
  // A leftover from an earlier run must not survive into the deploy.
  fs.mkdirSync(path.join(target, "webgl2-new"), { recursive: true });
  fs.writeFileSync(path.join(target, "webgl2-new", "stale.wasm"), "stale");

  assert.deepStrictEqual(packageInto(target), [
    "build-manifest.json",
    "index.html",
    "loader.js",
    "session.js",
    "webgl2/collateral-game.js",
    "webgl2/collateral-game_bg.wasm",
    "webgl2/index.html",
    "webgpu/collateral-game.js",
    "webgpu/collateral-game_bg.wasm",
    "webgpu/index.html",
  ]);
});

test("never ships compressed sidecars, source maps or experimental builds", () => {
  const target = fs.mkdtempSync(path.join(os.tmpdir(), "site-"));
  const files = packageInto(target);
  const forbidden = files.filter((file) => /\.(br|gz|map)$/.test(file) || /-new|-cpu/.test(file));
  assert.deepStrictEqual(forbidden, []);
  // The .br sidecars exist in dist; excluding them is a decision, not an accident.
  assert.ok(fs.existsSync(path.join(dist, "webgl2", "collateral-game_bg.wasm.br")));
});

test("copies the manifest that the loader reads", () => {
  const target = fs.mkdtempSync(path.join(os.tmpdir(), "site-"));
  packageInto(target);
  const manifest = JSON.parse(fs.readFileSync(path.join(target, "build-manifest.json"), "utf8"));
  assert.ok(manifest.webgl2 && manifest.webgpu, "manifest keeps both renderers");
  for (const renderer of ["webgl2", "webgpu"]) {
    const reference = manifest[renderer];
    assert.ok(fs.existsSync(path.join(target, reference.js)), `${renderer} js is packaged`);
  }
});
