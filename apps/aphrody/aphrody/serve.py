# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Static / SPA web server (+ optional reverse proxy) for a Linux VPS.

Serves a built site (e.g. a TypeScript React ``dist/``) with **single-page-app
fallback** (unknown routes return ``index.html`` so client-side routing works),
optional **in-RAM file caching** (preload the whole site into memory — exploits
available RAM for zero-disk-hit serving), threaded concurrency, and an optional
**reverse proxy** so one process can serve the static front-end *and* forward an
API prefix (e.g. ``/api``) to a backend such as a Rust app. Pure stdlib, no
third-party dependency, designed to run under **systemd**.

Paths and bind address are fully configurable (no hardcoding); the default host
is ``0.0.0.0`` for a VPS.

    aphrody serve /srv/site --host 0.0.0.0 --port 8080 --cache
    aphrody serve /srv/site --proxy http://127.0.0.1:3000 --proxy-prefix /api
"""

from __future__ import annotations

import logging
import mimetypes
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlsplit

logger = logging.getLogger(__name__)

#: VPS default: bind all interfaces (front with nginx/Caddy if you want TLS).
DEFAULT_HOST = "0.0.0.0"
DEFAULT_PORT = 8080

#: Filenames whose responses must never be cached by the browser (the SPA shell).
_NO_CACHE = frozenset({"index.html", "service-worker.js", "sw.js"})

#: Hop-by-hop headers not forwarded across the reverse proxy (RFC 7230 §6.1)
#: plus content-length/host which are recomputed per leg.
_HOP_BY_HOP = frozenset(
    {
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        "content-length",
        "host",
    }
)


def _preload(root: Path) -> dict[str, bytes]:
    """Read every file under *root* into a memory cache (RAM exploitation)."""
    cache: dict[str, bytes] = {}
    total = 0
    for path in root.rglob("*"):
        if path.is_file():
            data = path.read_bytes()
            cache[str(path)] = data
            total += len(data)
    logger.info(
        "preloaded %d files (%.1f MB) into RAM", len(cache), total / 1e6
    )
    return cache


def make_server(
    root: str | Path,
    host: str = DEFAULT_HOST,
    port: int = DEFAULT_PORT,
    *,
    spa: bool = True,
    cache: bool = False,
    proxy: str | None = None,
    proxy_prefix: str = "/api",
) -> ThreadingHTTPServer:
    """Build a threaded static/SPA HTTP server rooted at *root*.

    Args:
        root: Directory of the built site to serve (e.g. the React ``dist``).
        host: Bind address (``0.0.0.0`` by default).
        port: Bind port.
        spa: Serve ``index.html`` for unknown non-file routes (client routing).
        cache: Preload all files into RAM and serve from memory.
        proxy: Optional backend base URL (e.g. ``http://127.0.0.1:3000``);
            requests under *proxy_prefix* are forwarded to it.
        proxy_prefix: URL prefix routed to the backend (default ``/api``).

    Returns:
        A bound :class:`ThreadingHTTPServer` (call ``serve_forever``).

    Raises:
        FileNotFoundError: If *root* is not a directory.
    """
    root_p = Path(root).resolve()
    if not root_p.is_dir():
        raise FileNotFoundError(f"site root is not a directory: {root_p}")
    store = _preload(root_p) if cache else None
    index = root_p / "index.html"
    backend = proxy.rstrip("/") if proxy else None
    prefix = "/" + proxy_prefix.strip("/")

    class _Handler(BaseHTTPRequestHandler):
        server_version = "aphrody-serve/1.0"
        protocol_version = "HTTP/1.1"

        def log_message(self, fmt: str, *args: Any) -> None:
            logger.info("%s %s", self.address_string(), fmt % args)

        # -- static / SPA ------------------------------------------------
        def _resolve(self) -> Path | None:
            rel = unquote(urlsplit(self.path).path).lstrip("/")
            target = (root_p / rel).resolve()
            try:  # block path traversal outside the root
                target.relative_to(root_p)
            except ValueError:
                return None
            if target.is_dir():
                target = target / "index.html"
            return target

        def _read(self, target: Path) -> bytes:
            key = str(target)
            if store is not None:
                cached = store.get(key)
                if cached is not None:
                    return cached
            data = target.read_bytes()
            if store is not None:
                store[key] = data
            return data

        def _respond(self, *, head: bool) -> None:
            target = self._resolve()
            if target is None:
                self.send_error(403, "Forbidden")
                return
            if not target.is_file():
                if spa and index.is_file():
                    target = index
                else:
                    self.send_error(404, "Not Found")
                    return
            try:
                data = self._read(target)
            except OSError:
                self.send_error(404, "Not Found")
                return
            ctype = (
                mimetypes.guess_type(str(target))[0]
                or "application/octet-stream"
            )
            self.send_response(200)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(data)))
            if target.name in _NO_CACHE:
                self.send_header("Cache-Control", "no-cache")
            else:
                # Built assets are content-hashed → safe to cache hard.
                self.send_header(
                    "Cache-Control", "public, max-age=31536000, immutable"
                )
            self.end_headers()
            if not head:
                self.wfile.write(data)

        # -- reverse proxy -----------------------------------------------
        def _is_proxied(self) -> bool:
            if backend is None:
                return False
            path = urlsplit(self.path).path
            return path == prefix or path.startswith(prefix + "/")

        def _proxy(self) -> None:
            length = int(self.headers.get("Content-Length", 0) or 0)
            body = self.rfile.read(length) if length else None
            fwd = {
                k: v
                for k, v in self.headers.items()
                if k.lower() not in _HOP_BY_HOP
            }
            fwd["X-Forwarded-For"] = self.client_address[0]
            fwd["X-Forwarded-Host"] = self.headers.get("Host", "")
            fwd["X-Forwarded-Proto"] = "http"
            req = urllib.request.Request(
                backend + self.path, data=body, method=self.command, headers=fwd
            )
            try:
                with urllib.request.urlopen(req, timeout=60) as resp:
                    self._relay(resp.status, resp.headers, resp.read())
            except urllib.error.HTTPError as exc:  # backend 4xx/5xx — relay it
                self._relay(exc.code, exc.headers, exc.read())
            except (urllib.error.URLError, OSError) as exc:
                self.send_error(502, f"Bad Gateway: {exc}")

        def _relay(self, status: int, headers: Any, data: bytes) -> None:
            self.send_response(status)
            for k, v in headers.items():
                if k.lower() not in _HOP_BY_HOP:
                    self.send_header(k, v)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(data)

        # -- method dispatch ---------------------------------------------
        def do_GET(self) -> None:
            self._proxy() if self._is_proxied() else self._respond(head=False)

        def do_HEAD(self) -> None:
            self._proxy() if self._is_proxied() else self._respond(head=True)

        def do_POST(self) -> None:
            self._proxy() if self._is_proxied() else self.send_error(405)

        def do_PUT(self) -> None:
            self._proxy() if self._is_proxied() else self.send_error(405)

        def do_DELETE(self) -> None:
            self._proxy() if self._is_proxied() else self.send_error(405)

        def do_PATCH(self) -> None:
            self._proxy() if self._is_proxied() else self.send_error(405)

    httpd = ThreadingHTTPServer((host, port), _Handler)
    httpd.daemon_threads = True
    return httpd


def serve(
    root: str | Path,
    host: str = DEFAULT_HOST,
    port: int = DEFAULT_PORT,
    *,
    spa: bool = True,
    cache: bool = False,
    proxy: str | None = None,
    proxy_prefix: str = "/api",
) -> None:
    """Run the static/SPA server (with optional proxy) until interrupted.

    Args:
        root: Directory of the built site.
        host: Bind address.
        port: Bind port.
        spa: SPA index fallback.
        cache: Preload into RAM.
        proxy: Optional backend base URL to forward *proxy_prefix* to.
        proxy_prefix: URL prefix routed to the backend (default ``/api``).
    """
    httpd = make_server(
        root,
        host,
        port,
        spa=spa,
        cache=cache,
        proxy=proxy,
        proxy_prefix=proxy_prefix,
    )
    logger.info(
        "aphrody serving %s on http://%s:%d (spa=%s cache=%s proxy=%s)",
        root,
        host,
        port,
        spa,
        cache,
        proxy or "off",
    )
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:  # pragma: no cover - interactive
        pass
    finally:
        httpd.server_close()
