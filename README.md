# COLLATERAL

Early M0 platform spike for a single-player, static-site Rust/Bevy project. **Not yet a playable game.** Ballistics and injury, when implemented, will be stylised rather than medically accurate.

## Current status

See [PLAN.md](PLAN.md) for gates and design decisions and [PERF.md](PERF.md) for what has and has not been measured. No character asset has been accepted. iPhone Safari, renderer fallback and GitHub Pages have not been verified.

## Build

Requires Rust 1.99.1 with `wasm32-unknown-unknown`, Trunk 0.22.1, Node.js and network access to the Rust crate/build-tool registries. The pinned toolchain is specified by `rust-toolchain.toml`.

```sh
bash tools/build-web.sh /SE-Browser-Remake/
```

For local static hosting, build with `/` instead. **The complete build currently fails its conservative uncompressed 8 MB M0 gate**, so it does not yet produce a complete deployable distribution. Individual WebGL2 and WebGPU Trunk builds succeeded; only WebGL2 rendered in desktop Chrome. WebGPU currently produces a blank canvas in that environment. GitHub Pages determines HTTP content encoding; `.br` files do not prove actual transfer size. Renderer overrides are `?renderer=webgl2` and `?renderer=webgpu`; `?diag=1` requests the Bevy update-FPS overlay.

There is no service, login or multiplayer. Gameplay, level, models, audio and cinematic systems are not implemented yet.
