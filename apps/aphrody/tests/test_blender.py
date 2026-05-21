# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.blender` against a mock blender-mcp socket server."""

from __future__ import annotations

import contextlib
import json
import socket
import threading
from collections.abc import Callable

import pytest
from aphrody.blender import BlenderClient, BlenderError


@contextlib.contextmanager
def mock_addon(responder: Callable[[dict], dict]):
    """Run a one-connection TCP server mimicking the blender-mcp addon.

    *responder* maps a received command dict to a response dict. Yields the
    bound port.
    """
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("localhost", 0))
    port = srv.getsockname()[1]
    srv.listen(1)

    def serve() -> None:
        try:
            conn, _ = srv.accept()
        except OSError:
            return
        buffer = b""
        with conn:
            while True:
                try:
                    data = conn.recv(8192)
                except OSError:
                    break
                if not data:
                    break
                buffer += data
                try:
                    cmd = json.loads(buffer.decode("utf-8"))
                    buffer = b""
                except json.JSONDecodeError:
                    continue
                conn.sendall(json.dumps(responder(cmd)).encode("utf-8"))

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    try:
        yield port
    finally:
        srv.close()


def test_get_scene_info_success() -> None:
    def responder(cmd: dict) -> dict:
        assert cmd["type"] == "get_scene_info"
        return {"status": "success", "result": {"object_count": 3}}

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        assert bl.get_scene_info() == {"object_count": 3}


def test_execute_code_returns_stdout() -> None:
    def responder(cmd: dict) -> dict:
        assert cmd["type"] == "execute_code"
        assert "code" in cmd["params"]
        return {
            "status": "success",
            "result": {"executed": True, "result": "hi\n"},
        }

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        assert bl.execute_code("print('hi')") == "hi\n"


def test_eval_json_parses_last_line() -> None:
    def responder(_cmd: dict) -> dict:
        out = 'noise line\n["Cube", "Lamp"]\n'
        return {
            "status": "success",
            "result": {"executed": True, "result": out},
        }

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        assert bl.eval_json("...") == ["Cube", "Lamp"]


def test_import_glb_returns_new_objects() -> None:
    def responder(cmd: dict) -> dict:
        assert cmd["type"] == "execute_code"
        return {
            "status": "success",
            "result": {"executed": True, "result": '["aphrody"]'},
        }

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        assert bl.import_glb("model.glb") == ["aphrody"]


def test_error_status_raises() -> None:
    def responder(_cmd: dict) -> dict:
        return {"status": "error", "message": "boom"}

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        with pytest.raises(BlenderError, match="boom"):
            bl.get_scene_info()


def test_connect_failure_raises() -> None:
    # Bind then close to obtain a port nobody is listening on.
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(("localhost", 0))
    port = s.getsockname()[1]
    s.close()
    with pytest.raises(BlenderError, match="could not connect"):
        BlenderClient("localhost", port, timeout=2.0).get_scene_info()


def test_pro_commands_stats_and_optimize() -> None:
    def responder(cmd: dict) -> dict:
        if cmd["type"] == "aphrody_scene_stats":
            return {"status": "success", "result": {"triangles": 1234}}
        if cmd["type"] == "aphrody_optimize_mesh":
            assert cmd["params"]["decimate_ratio"] == 0.5
            return {
                "status": "success",
                "result": {"optimized": [{"object": "Cube"}]},
            }
        return {"status": "error", "message": "unexpected"}

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        assert bl.scene_stats()["triangles"] == 1234
        report = bl.optimize_mesh(decimate_ratio=0.5)
        assert report["optimized"][0]["object"] == "Cube"


def test_chunked_response_reassembled() -> None:
    """A response split across recv() calls must still parse."""
    big = {"status": "success", "result": {"blob": "x" * 20000}}

    def responder(_cmd: dict) -> dict:
        return big

    with mock_addon(responder) as port, BlenderClient("localhost", port) as bl:
        assert bl.get_scene_info()["blob"] == "x" * 20000
