#!/usr/bin/env python3
"""End-to-end M0 check in a real browser.

Proves three things a build-script size report cannot:
  1. the WebGL2 artifact initializes and reports no renderer error;
  2. the canvas contains a rendered, animated 3D scene (pixels, not an update loop);
  3. a cold first load transfers less than the M0 8 MB budget over Brotli.

The scene draws 1,000 teal cubes on a dark background, so the checks assert a dark
background, a large teal-shaded share and frame-to-frame change.
"""
import argparse
import io
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "dist"
VIEWPORT = {"width": 390, "height": 844}
STARTUP_TIMEOUT_MS = 120_000

failures = []


def check(condition, message):
    if condition:
        print(f"  ok   {message}")
    else:
        print(f"  FAIL {message}")
        failures.append(message)


def is_teal(pixel):
    r, g, b = pixel[0], pixel[1], pixel[2]
    # Material base color is srgb(0.32, 0.76, 0.72): green and blue dominate red.
    return g > r + 20 and b > r + 10 and g > 70


def is_dark(pixel):
    return max(pixel[0], pixel[1], pixel[2]) < 60


def analyse(png_bytes):
    image = Image.open(io.BytesIO(png_bytes)).convert("RGB")
    width, height = image.size
    # Sample every 2nd pixel: the canvas is large and the cubes are big enough.
    pixels = [image.getpixel((x, y)) for y in range(0, height, 2) for x in range(0, width, 2)]
    total = len(pixels)
    return {
        "size": (width, height),
        "distinct": len(set(pixels)),
        "teal": sum(1 for p in pixels if is_teal(p)) / total,
        "dark": sum(1 for p in pixels if is_dark(p)) / total,
        "pixels": pixels,
    }


def changed_fraction(before, after):
    pairs = list(zip(before["pixels"], after["pixels"]))
    if not pairs:
        return 0.0
    # Compare with a small tolerance so tonemapping noise alone is not "animation".
    different = sum(1 for a, b in pairs if max(abs(a[i] - b[i]) for i in range(3)) > 8)
    return different / len(pairs)


def start_server(page_url):
    if page_url:
        return None, page_url
    process = subprocess.Popen(
        [sys.executable, str(ROOT / "tools" / "serve-dist.py"), "--port", "0"],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    for _ in range(200):
        line = process.stdout.readline()
        if not line:
            break
        if line.startswith("SERVING "):
            return process, line.split()[1].strip()
    process.terminate()
    raise RuntimeError("local static server did not report a URL")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default="", help="serve dist yourself and pass the URL")
    parser.add_argument("--headed", action="store_true")
    args = parser.parse_args()

    server, base = start_server(args.base_url)
    print(f"base: {base}")
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=not args.headed)
            context = browser.new_context(viewport=VIEWPORT, device_scale_factor=1)
            page = context.new_page()
            console = []
            page.on("console", lambda msg: console.append(f"{msg.type}: {msg.text}"))
            page.on("pageerror", lambda err: console.append(f"pageerror: {err}"))

            cdp = context.new_cdp_session(page)
            received = []
            cdp.on("Network.loadingFinished", lambda event: received.append(event["encodedDataLength"]))
            cdp.send("Network.enable")
            cdp.send("Network.setCacheDisabled", {"cacheDisabled": True})

            page.goto(f"{base}?renderer=webgl2&diag=1", wait_until="domcontentloaded")
            page.wait_for_function(
                "() => document.documentElement.dataset.bevyRunning === 'true'"
                " || document.documentElement.dataset.renderError",
                timeout=STARTUP_TIMEOUT_MS,
            )

            print("startup")
            renderer = page.evaluate("() => document.documentElement.dataset.renderer")
            error = page.evaluate("() => document.documentElement.dataset.renderError || null")
            check(renderer == "WEBGL2", f"selected the WebGL2 artifact (got {renderer})")
            check(error is None, f"no renderer error was reported (got {error})")

            # Give the scene a few frames so an early capture is not an empty clear.
            page.wait_for_timeout(4000)
            check(page.locator("#loading").is_hidden(), "the loading overlay is hidden")

            canvas = page.locator("#game")
            box = canvas.bounding_box()
            check(bool(box) and box["width"] > 100 and box["height"] > 100,
                  f"the canvas has a visible size ({box})")

            print("pixels")
            first = analyse(canvas.screenshot())
            # A flat fill has a handful of colours; depth-shaded cubes produce dozens.
            check(first["distinct"] > 16,
                  f"frame is shaded rather than a flat fill (distinct={first['distinct']})")
            check(first["dark"] > 0.05, f"frame is not a blank clear (dark share={first['dark']:.3f})")
            check(first["teal"] > 0.10,
                  f"teal cube material covers the viewport (share={first['teal']:.3f})")

            page.wait_for_timeout(1500)
            second = analyse(canvas.screenshot())
            moved = changed_fraction(first, second)
            check(moved > 0.01, f"the scene animates between captures (changed={moved:.3f})")

            print("budget")
            # Re-load cold now that every asset is warm on the server but not in the browser.
            received.clear()
            page.goto(f"{base}?renderer=webgl2&diag=1", wait_until="load")
            page.wait_for_function(
                "() => document.documentElement.dataset.bevyRunning === 'true'",
                timeout=STARTUP_TIMEOUT_MS,
            )
            page.wait_for_timeout(1000)
            transferred = sum(received)
            check(transferred < 8_000_000,
                  f"cold first load transferred {transferred} bytes (< 8000000, Brotli)")
            check(transferred > 1_000_000,
                  f"the transfer figure covers the wasm payload ({transferred} bytes)")

            errors = [line for line in console if line.startswith(("error", "pageerror"))]
            check(not errors, f"no console errors ({errors[:3]})")

            print("overrides")
            # An unknown override must fail visibly instead of loading a renderer.
            page.goto(f"{base}?renderer=bogus", wait_until="domcontentloaded")
            page.wait_for_function(
                "() => document.querySelector('#status').textContent.includes('Unknown renderer')",
                timeout=15_000,
            )
            check(not page.locator("#loading").is_hidden(), "the loading panel stays up on failure")
            check(page.locator("#retry").is_visible(), "a reload action is offered on failure")
            check(page.evaluate("() => document.documentElement.dataset.renderer === undefined"),
                  "no renderer artifact is selected for an unknown override")
            browser.close()
    finally:
        if server:
            server.terminate()
            server.wait(timeout=10)

    print()
    if failures:
        print(f"{len(failures)} check(s) failed")
        return 1
    print("All browser checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
