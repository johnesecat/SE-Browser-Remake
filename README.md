# COLLATERAL

Early M0 platform spike for a single-player, static-site Rust/Bevy project. **Not yet a playable game.** Ballistics and injury, when implemented, will be stylised rather than medically accurate.

## Current status

See [TASKS.md](TASKS.md) for the durable work checklist, [PLAN.md](PLAN.md) for gates and design decisions, and [PERF.md](PERF.md) for what has and has not been measured. No character asset has been accepted. iPhone Safari and GitHub Pages have not been verified.

## Build

Requires Rust 1.99.1 with `wasm32-unknown-unknown`, Trunk 0.22.1, Node.js and network access to the Rust crate/build-tool registries. The pinned toolchain is specified by `rust-toolchain.toml`.

```sh
bash tools/build-web.sh /SE-Browser-Remake/
```

For local static hosting, build with `/` instead. The build finishes by running `tools/check-budget.js`, which gates the M0 first-load budget on Brotli bytes (under 8 MB) and the wasm ceiling (12 MB Brotli), and reports uncompressed sizes for diagnosis. Current artifacts are 2.90 MB (WebGL2) and 2.43 MB (WebGPU) Brotli, so the gate passes; uncompressed sizes are 13.5 MB and 11.9 MB. WebGL2 renders, while WebGPU loses its device in cloud Chrome; automatic selection retries once with WebGL2 on a fresh page, and a forced renderer reports the error. Renderer overrides are `?renderer=webgl2` and `?renderer=webgpu`; `?diag=1` requests the Bevy update-FPS overlay.

## Tests

```sh
node tools/test-loader.js   # watchdog, hidden-tab time, render error and fallback
node tools/test-budget.js   # first-load and wasm budget boundaries
python3 tools/test-web.py   # rendered pixels and cold-load bytes in a real browser
```

`bash tools/ci-test.sh` runs all of them in the order CI uses and stops at the first failure; add `--skip-browser` where Chromium is unavailable. The GitHub Actions workflow (`.github/workflows/ci.yml`) builds both artifacts, runs that script, packages `_site` and deploys it to Pages on `main` only. Nothing has been deployed from this workspace, and the workflow has never run on GitHub: Pages also has to be switched to the GitHub Actions source in repository settings before a deploy can succeed.

`tools/test-web.py` needs Playwright with Chromium plus Pillow. It starts `tools/serve-dist.py`, which serves `dist/` with the Brotli/gzip negotiation GitHub Pages performs, then asserts the WebGL2 canvas shows a shaded, animated teal cube scene and that a cold load transfers under 8 MB. The committed `.br` files are not proof of Pages' negotiated encoding; the local figure is not a Pages measurement.

This workspace's system Python is read-only and the image lacks Chromium's system libraries, so the browser test was set up with a project-local environment:

```sh
python3 -m venv .tools/venv && .tools/venv/bin/pip install playwright pillow
.tools/venv/bin/python -m playwright install chromium
# Chromium needs libraries this image does not ship; unpack them without root.
LD_LIBRARY_PATH=$PWD/.tools/debs/root/usr/lib/x86_64-linux-gnu .tools/venv/bin/python tools/test-web.py
```

The unpacked libraries live under the ignored `.tools/` directory, so re-run the package step after a workspace restore.

There is no service, login or multiplayer. Gameplay, level, models, audio and cinematic systems are not implemented yet.
