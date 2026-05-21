# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.serve` (threaded static / SPA server, stdlib)."""

from __future__ import annotations

import threading
import urllib.error
import urllib.request

import pytest
from aphrody import serve


def _start(httpd) -> int:
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd.server_address[1]


def _get(port: int, path: str) -> tuple[int, bytes]:
    with urllib.request.urlopen(
        f"http://127.0.0.1:{port}{path}", timeout=5
    ) as r:
        return r.status, r.read()


def test_serves_asset_and_spa_fallback(tmp_path) -> None:
    (tmp_path / "index.html").write_text("<html>home</html>", encoding="utf-8")
    (tmp_path / "app.js").write_text("console.log(1)", encoding="utf-8")
    httpd = serve.make_server(tmp_path, "127.0.0.1", 0, spa=True, cache=True)
    port = _start(httpd)
    try:
        status, body = _get(port, "/app.js")
        assert status == 200
        assert body == b"console.log(1)"
        # Unknown client-side route → index.html (SPA fallback).
        status, body = _get(port, "/some/deep/client/route")
        assert status == 200
        assert b"home" in body
    finally:
        httpd.shutdown()
        httpd.server_close()


def test_404_without_spa(tmp_path) -> None:
    (tmp_path / "index.html").write_text("x", encoding="utf-8")
    httpd = serve.make_server(tmp_path, "127.0.0.1", 0, spa=False)
    port = _start(httpd)
    try:
        with pytest.raises(urllib.error.HTTPError) as exc:
            _get(port, "/missing")
        assert exc.value.code == 404
    finally:
        httpd.shutdown()
        httpd.server_close()


def test_bad_root_raises(tmp_path) -> None:
    with pytest.raises(FileNotFoundError):
        serve.make_server(tmp_path / "does-not-exist")


def test_reverse_proxy_forwards_and_static_falls_through(tmp_path) -> None:
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    (tmp_path / "index.html").write_text("home", encoding="utf-8")

    class _Backend(BaseHTTPRequestHandler):
        def log_message(self, *_a) -> None:
            pass

        def do_GET(self) -> None:
            body = b'{"path":"' + self.path.encode() + b'"}'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self) -> None:
            n = int(self.headers.get("Content-Length", 0) or 0)
            data = self.rfile.read(n)
            self.send_response(201)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    backend = ThreadingHTTPServer(("127.0.0.1", 0), _Backend)
    backend.daemon_threads = True
    be_port = _start(backend)
    httpd = serve.make_server(
        tmp_path,
        "127.0.0.1",
        0,
        proxy=f"http://127.0.0.1:{be_port}",
        proxy_prefix="/api",
    )
    port = _start(httpd)
    try:
        status, body = _get(port, "/api/hello")
        assert status == 200
        assert b"/api/hello" in body  # backend saw the full path
        # Non-proxied route → static SPA shell.
        status, body = _get(port, "/dashboard")
        assert status == 200 and b"home" in body
        # POST is proxied (echoed by the backend).
        req = urllib.request.Request(
            f"http://127.0.0.1:{port}/api/echo", data=b"payload", method="POST"
        )
        with urllib.request.urlopen(req, timeout=5) as r:
            assert r.read() == b"payload"
    finally:
        httpd.shutdown()
        httpd.server_close()
        backend.shutdown()
        backend.server_close()


def test_preload_caches_files(tmp_path) -> None:
    (tmp_path / "a.txt").write_text("hello", encoding="utf-8")
    cache = serve._preload(tmp_path)
    assert cache[str((tmp_path / "a.txt").resolve())] == b"hello" or any(
        v == b"hello" for v in cache.values()
    )
