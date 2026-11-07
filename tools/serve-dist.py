#!/usr/bin/env python3
"""Serve dist/ the way GitHub Pages does: same-origin static files, Brotli preferred.

GitHub Pages compresses text assets itself, so a plain http.server (which never
compresses) cannot measure the real first load. This serves the precompressed
`.br` / `.gz` sidecars produced by tools/check-budget.js when the browser accepts
them, and the identity file otherwise.

Prints one line, `SERVING <url>`, once listening, so tests can wait for readiness.
"""
import argparse
import http.server
import os
import socket
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_ROOT = ROOT / "dist"

# GitHub Pages derives the type from the file extension; .wasm must be application/wasm.
CONTENT_TYPES = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".mjs": "text/javascript; charset=utf-8",
    ".json": "application/json",
    ".wasm": "application/wasm",
    ".css": "text/css; charset=utf-8",
    ".png": "image/png",
    ".svg": "image/svg+xml",
    ".ico": "image/x-icon",
}


class Handler(http.server.BaseHTTPRequestHandler):
    root = DEFAULT_ROOT
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def do_GET(self):  # noqa: N802 - stdlib naming
        self.respond(body=True)

    def do_HEAD(self):  # noqa: N802 - stdlib naming
        self.respond(body=False)

    def resolve(self):
        path = self.path.split("?", 1)[0].split("#", 1)[0]
        if path.endswith("/"):
            path += "index.html"
        candidate = (self.root / path.lstrip("/")).resolve()
        # Refuse to escape the dist directory via ../ in the request path.
        if not str(candidate).startswith(str(self.root.resolve())) or not candidate.is_file():
            return None
        return candidate

    def accepted_encoding(self, file_path):
        header = self.headers.get("Accept-Encoding", "")
        accepted = {part.split(";")[0].strip().lower() for part in header.split(",")}
        for encoding, suffix in (("br", ".br"), ("gzip", ".gz")):
            sidecar = file_path.with_name(file_path.name + suffix)
            if encoding in accepted and sidecar.is_file():
                return encoding, sidecar
        return None, file_path

    def respond(self, body):
        file_path = self.resolve()
        if file_path is None:
            self.send_error(404)
            return
        encoding, payload_path = self.accepted_encoding(file_path)
        payload = payload_path.read_bytes()
        self.send_response(200)
        content_type = CONTENT_TYPES.get(file_path.suffix, "application/octet-stream")
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("Cache-Control", "max-age=600")
        # Required so a cache cannot serve a Brotli body to a client that lacks support.
        self.send_header("Vary", "Accept-Encoding")
        if encoding:
            self.send_header("Content-Encoding", encoding)
        self.end_headers()
        if body:
            self.wfile.write(payload)


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=3000)
    parser.add_argument("--root", default=str(DEFAULT_ROOT))
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()

    Handler.root = Path(args.root)
    server = Server((args.host, args.port), Handler)
    port = server.server_address[1]
    print(f"SERVING http://127.0.0.1:{port}/", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    sys.exit(main())
