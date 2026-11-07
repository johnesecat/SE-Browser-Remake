# Performance report

M0 remains **incomplete**. These are local spike observations, not iPhone results or acceptance passes. Measured 2026-11-07 in the Freebuff cloud workspace, Rust 1.99.1, Bevy 0.20.1, Trunk 0.22.1, `wasm-opt` version_131 `-Oz`. Brotli quality 11 and gzip level 9 were measured locally with Node 26.13.0. WebGL2 artifact includes the wall-time diagnostic and ACES camera correction; the WebGPU artifact was built while a subsequent lint-only source edit was being made.

| Measurement | Environment | Result |
| --- | --- | --- |
| WebGL2 wasm | local optimized artifact | 13,523,653 bytes uncompressed; 2,781,795 bytes Brotli; 4,188,083 bytes gzip |
| WebGPU wasm | local optimized artifact | 11,938,806 bytes uncompressed; 2,309,981 bytes Brotli; 3,542,038 bytes gzip |
| WebGL2 scene | cloud browser, Chrome 154, 1024×768; reported ANGLE / Intel UHD 630 | 1,000 cubes rendered; sampled overlay around 2.3–2.4 Bevy updates/s (~410–433 ms per update); not a controlled benchmark, no phone inference |
| WebGPU scene | same cloud browser | adapter probe passed but the Bevy rendering device was lost (Chrome reported its external instance was unavailable); canvas-format performance warning; root cause unknown; **not working/verified** |
| Automatic fallback | cloud Chrome, local static server | after WebGPU device loss, a fresh page loaded the WebGL2 artifact and its update loop ran; forced WebGPU showed the device-loss error; actual phone fallback not tested |
| WebGPU isolation | cloud Chrome, local static server | a standalone offscreen triangle produced the expected RGBA pixel `[64, 128, 191, 255]`; Bevy still lost its device when the loader's probe-device creation/destruction was removed |
| CPU mesh preprocessing experiment | same cloud browser, WebGPU release artifact | setting Bevy's `use_gpu_instance_buffer_builder` to `false` did not prevent device loss; experimental change reverted |
| First-load network bytes / 4G time | GitHub Pages / phone | not measured |
| Frame times by scene and tier | iPhone | not measured |
| Draw calls / triangles / GPU time | any | not measured |
| Peak memory / texture residency | any | not measured |
| 45-minute soak / crashes | any | not measured |
| iPhone 15 Safari rotation/background | iPhone | not measured |

The WebGPU isolation tests used a separate, simple offscreen render, not the Bevy scene or a canvas-presentation test. Bevy's device-loss cause remains unidentified; the trace showed environment-map downsampling and other pipelines being initialized, but did not establish which operation caused the loss. Chromium's [maintainers note](https://github.com/gpuweb/gpuweb/discussions/4676) that the external-instance message alone does not identify a cause. The cloud browser intermittently returned 503s or lost its connection during further isolation. The loader's Bevy update-loop signal is not proof that a frame has rendered. A synthetic `webglcontextlost` event displayed the reload error; an actual WebGL context loss was not tested. The fallback downloads a second renderer after a failed WebGPU attempt, so the first-load target needs separate measurement for that path. The overlay reports Bevy update-frame wall time and FPS, not GPU time, draw calls or memory. A renderer label describes the selected artifact, not independent backend verification. The first release build failed because Binaryen rejected bulk-memory instructions; the retry passed after explicitly enabling the relevant Wasm features. wasm-bindgen warns that Rust's `strip = true` removed `target_features` metadata; this must be resolved and both builds retested. A conservative uncompressed 8 MB gate currently fails for both artifacts. Pages' actual negotiated encoding and cold first-load size remain unverified.
