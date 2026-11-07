# COLLATERAL — durable task ledger

Update this file after each meaningful increment. `[x]` means implemented or observed as described, **not** that a whole milestone is accepted. `[ ]` means unfinished, unverified, or blocked. Keep `PLAN.md` for architectural decisions, `PERF.md` for measured evidence, and `ASSETS.md` / `CREDITS.md` for provenance and attribution. Never infer a phone or GitHub Pages pass from a desktop preview.

## Checkpoint and constraints

- [x] User reports the project loads in the cloud browser at this checkpoint. The renderer, visible scene, performance, URL, and phone behavior were not independently established by that report.
- [x] Rust 1.99.1, Bevy 0.20.1, Trunk 0.22.1, wasm-bindgen 0.2.130 and wasm-opt version_131 are pinned. Static single-player Rust/Bevy, single-threaded wasm, no backend, CDN-hosted wasm, authentication or multiplayer.
- [x] The user previously authorized sourcing a character and a public personal-project demo; no character has been accepted. Review each asset's license and provenance before use; do not treat a personal project as a license. Z-Anatomy remains only a candidate with potentially restricted components.
- [x] Requested minimum target: iPhone 15 and iOS 27.2; availability and performance are unverified. Gore defaults off/opt-in pending a decision. Original brief's older-device targets remain aspirational until reconciled with that later minimum.
- [ ] Keep outcomes deterministic and resolved before cinematic replay; pure-Rust simulation, data-driven tuning and measured performance. Never invent assets, measurements, renderer success or tests.
- [ ] Work in milestone order; finish M0 before starting M1. Desktop Chrome may establish preliminary M0–M1 evidence but must never be represented as an iPhone pass.

## M0 — platform spike (ACTIVE; not accepted)

### Implemented / observed
- [x] Separate WebGPU and WebGL2 feature builds, loader/overrides, progress UI, diagnostics, debounced viewport resizing, and a 1,000-spinning-cube Bevy scene.
- [x] Desktop Chrome previously rendered the WebGL2 scene; a WebGPU device loss was reproduced in cloud Chrome. The user now reports a successful cloud-browser load, with exact renderer and pixels not independently verified at this checkpoint.
- [x] Auto-selected WebGPU device loss retries WebGL2 once in a fresh page; forced WebGPU exposes an error. An update-loop signal does **not** prove visible pixels. Actual context loss and iPhone fallback remain unverified.
- [x] Individual release builds and strict wasm Clippy checks have succeeded. Local compressed artifact sizes and prior desktop observations are recorded in `PERF.md`; local `.br` sizes are not network-transfer measurements.
- [x] Isolated a cloud Chrome canvas-path failure without Bevy and separately observed a storage-texture validation mismatch; the browser/driver root cause remains unknown. See `PERF.md`.

### Remaining work and acceptance
- [x] Build output reports both artifacts, hashes and both conservative size-gate results and writes the manifest even on budget failure; the original 8 MB gate remains enforced. Local `/` base path is accepted.
- [x] Preserve wasm `target_features` through wasm-bindgen by stripping debug information instead of all sections; both optimized artifacts rebuilt with no missing-metadata warning and pass Node's wasm validation. Cloud Chrome reached the WebGL2 update loop; forced WebGPU still reported device loss. See `PERF.md` for limitations.
- [x] Cover wasm initialization and Bevy update-loop stalls with a visible-time watchdog; focused Node checks cover hidden time, initialization rejection, success, render failure and bounded/forced fallback. This is not proof of rendered pixels.
- [x] Gate the M0 first-load budget on Brotli bytes (8 MB) rather than uncompressed bytes, keep the 12 MB Brotli wasm ceiling, and report uncompressed sizes as diagnostics. The gate lives in `tools/check-budget.js` with focused Node tests; `tools/build-web.sh` calls it. Decision recorded in `PLAN.md`; measured values in `PERF.md`.
- [x] Prove rendered pixels in a real browser: `tools/test-web.py` drives headless Chromium through Playwright against `tools/serve-dist.py`, asserts the WebGL2 scene is shaded, teal, animated between captures, error-free, and that a cold load transfers under 8 MB. A stub page that reports a running loop while drawing nothing fails 5 of these checks, so the assertions are not vacuous.
- [ ] **Next:** create GitHub Actions checks and Pages packaging for both artifacts, tests and gates, then verify WebGPU on an independent browser.
- [ ] Cover renderer selection, overrides, timeout and forced fallback in the browser test; the current browser test covers the WebGL2 happy path, the pixel proof and the transfer budget.
- [ ] Investigate WebGPU device loss on an independent real browser/device and address confirmed application causes. Do not infer a cause from Chromium's generic external-instance message.
- [ ] Create GitHub Actions checks and Pages packaging/deployment workflow for both builds, tests, lint, size gates and correct repository subpath; do not push/deploy without authorization.
- [ ] Measure actual cold-cache first-load transfer including manifest, loader, selected artifact and any fallback on GitHub Pages; target M0 <8 MB and wasm ≤12 MB Brotli. Local Brotli artifacts are 2.90 MB (WebGL2) and 2.43 MB (WebGPU), and a local browser load measured 2,897,310 bytes, but GitHub Pages' negotiated encoding is still unmeasured; do not claim a Pages pass.
- [ ] Desktop preliminary smoke: identify backend, verify both paths and visible 1,000-cube rendering, document actual errors and measured FPS. The WebGL2 path now has rendered-pixel evidence in headless Chromium; WebGPU, FPS and a hardware-GPU browser remain open.
- [ ] On iPhone Safari: live Pages URL, WebGPU ≥30 FPS, genuine WebGL2 fallback, cold 4G load, portrait/landscape rotation and background/foreground. Record device, OS, bytes, time and FPS; no pass until actually measured.

## M1 — character (blocked by M0 and an accepted asset)
- [ ] Source/receive a character legally usable in the public demo; verify exact provenance, license and hashes. The original brief preferred a supplied model; the later conversation authorized sourcing. Do not substitute or silently omit anatomy.
- [ ] Report format/size/triangles/materials/textures, skeleton/skin and all bone names, pose/axes/scale, separate bones/organs and missing features, shape keys and animation clips.
- [ ] Normalize canonical rig, scale, axes and feet origin; split body, named bone meshes and named organ meshes; map organs to bones; generate anatomy data and collision proxies.
- [ ] Bake/validate LODs, meshopt/KTX2 and budgets; procedural stand-ins only with explicit disclosure when needed. Publish file-level licenses, hashes, attributions and any share-alike obligations.
- [ ] Lit turntable and anatomy toggle; verify missing-organ, size, quality and performance acceptance on the appropriate device.

## M2 — deterministic ballistics and anatomy
- [ ] Pure-Rust crates and tests; data-driven weapons and tissue/organ/bone properties.
- [ ] 1 kHz seeded trajectory with drag, gravity, wind, Coriolis and zeroing; first-hit prediction; energy-conserving penetration, deflection, wound channel, fracture/damage and collateral.
- [ ] Damage HUD and tests: 1,000 deterministic replays, energy accounting and ordered chest hits, 100 scripted first-hit predictions.

## M3 — bullet camera
- [ ] Resolve shot first; data-driven camera/time tracks, chase/pre-impact/impact/exit/return, collateral chain, skip and hard timeout.
- [ ] Guard invalid vectors, destroyed target and clipping; force-abort every state and test 50 consecutive camera sequences with input restored.

## M4 — X-ray reveal
- [ ] Dithered skin dissolve, readable distinct anatomy, mobile uber-pass/grade and half-resolution outlines as budget allows.
- [ ] Hysteretic occlusion fade, at most eight meshes, impact/channel always visible; verify phone-sized readability and frame budget.

## M5 — wound channel and fracture
- [ ] Bake seeded bone-fracture variants and collision hulls; select using resolved channel and locally separate impacted fragments with bounded lifetime/count.
- [ ] Show wound channel, cavity, entry/exit wounds, organ states, fragments, blood and damage report consistent with resolved data.
- [ ] Verify location/state correctness, ≤40 active fragments per character and busiest-frame budget.

## M6 — active ragdoll, IK, behaviours and cloth
- [ ] Use the documented Rapier decision; anatomy-derived bodies/limits, PD joint motors, animation/physics blending, fixed 60 Hz bounded substeps and finite/velocity guards.
- [ ] Implement and use foot/arm IK, head look, weapon hold, procedural recoil and injury-aware behaviour/pain/consciousness states.
- [ ] Generate garments from body sections; bounded XPBD simulation, prewarm, collision and cloth LODs.
- [ ] Verify 45-second/10-corpse stability, injury-specific reactions and garment stability; no unsupported Euphoria equivalence claims.

## M7 — game
- [ ] Licensed outdoor level, cover/navigation and collateral sightline; time-sliced perception, utility AI, squads, cover and NPC lifecycle.
- [ ] Licensed or original weapons, aiming/scope, wind/range, breath/sway, controls and scoring/damage report.
- [ ] Touch-safe menu, pause, settings, credits, calibration and accessibility. Verify a 10-minute phone play session.

## M8 — polish
- [ ] Spatial/layered audio with first-touch unlock, slow-motion mix and silent-switch handling; camera feel, transitions and accessible subtitles.
- [ ] Versioned asset streaming and service-worker offline cache; dynamic resolution/tier fallback for thermal pressure.
- [ ] Measure ten-minute gameplay/cinematic frame times on target hardware.

## M9 — ship and cross-cutting
- [ ] CI: fmt, strict Clippy, workspace tests, both wasm builds and actual size gates, desktop smoke, and asset license/hash validation.
- [ ] Create device QA checklist with expected results and FPS slots: 4G cold load, kill cam/collateral, anatomy/occlusion/fracture/ragdoll/cloth, rotation, background ×10, 45-minute soak, low-power, skip every beat, audio, settings, cache-cleared and private reload.
- [ ] Finish `ASSETS.md`, in-game credits, `PERF.md` (actual bytes/time, scene FPS/frame times, draw calls, memory, crash count and unknowns) and concise README.
- [ ] Optimize, audit licenses and sizes, validate the actual published Pages URL from a clean phone, and document remaining limitations. No 45-minute stability claim without the run.
