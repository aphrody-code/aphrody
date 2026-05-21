# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.bpy_runner` (Blender resolution + run, mocked)."""

from __future__ import annotations

import subprocess
import types

import pytest
from aphrody import bpy_runner
from aphrody.bpy_runner import BlenderRunner, BlenderRunnerError, RunResult


def test_run_result_ok() -> None:
    assert RunResult(0, "", "").ok
    assert not RunResult(1, "", "boom").ok


def test_resolve_env_override(tmp_path, monkeypatch) -> None:
    fake = tmp_path / "blender.exe"
    fake.write_text("x", encoding="utf-8")
    monkeypatch.setenv("APHRODY_BLENDER_BIN", str(fake))
    assert bpy_runner.resolve_blender_bin() == str(fake)


def test_resolve_explicit_override(tmp_path) -> None:
    fake = tmp_path / "b.exe"
    fake.write_text("x", encoding="utf-8")
    assert bpy_runner.resolve_blender_bin(str(fake)) == str(fake)


def test_resolve_none_found(monkeypatch) -> None:
    monkeypatch.delenv("APHRODY_BLENDER_BIN", raising=False)
    monkeypatch.setattr(bpy_runner, "_KNOWN_BINARIES", ())
    monkeypatch.setattr(bpy_runner.shutil, "which", lambda _n: None)
    assert bpy_runner.resolve_blender_bin() is None


def test_runner_init_raises_without_blender(monkeypatch) -> None:
    monkeypatch.setattr(
        bpy_runner, "resolve_blender_bin", lambda override=None: None
    )
    with pytest.raises(BlenderRunnerError, match="no Blender binary"):
        BlenderRunner()


def test_run_script_invokes_blender(tmp_path, monkeypatch) -> None:
    fake_bin = tmp_path / "blender.exe"
    fake_bin.write_text("x", encoding="utf-8")
    script = tmp_path / "s.py"
    script.write_text("print('hi')", encoding="utf-8")

    captured = {}

    def fake_run(cmd, **kwargs):
        captured["cmd"] = cmd
        return types.SimpleNamespace(returncode=0, stdout="done\n", stderr="")

    monkeypatch.setattr(subprocess, "run", fake_run)
    runner = BlenderRunner(str(fake_bin))
    result = runner.run_script(script, ["--frames", 8])
    assert result.ok
    assert result.stdout == "done\n"
    # Correct headless invocation shape.
    assert captured["cmd"][:3] == [str(fake_bin), "-b", "--factory-startup"]
    assert "-P" in captured["cmd"]
    assert captured["cmd"][-3:] == ["--", "--frames", "8"]


def test_run_script_missing_script_raises(tmp_path) -> None:
    fake_bin = tmp_path / "blender.exe"
    fake_bin.write_text("x", encoding="utf-8")
    with pytest.raises(BlenderRunnerError, match="script not found"):
        BlenderRunner(str(fake_bin)).run_script(tmp_path / "nope.py")


def test_parse_gpu_json() -> None:
    out = (
        "noise line\n"
        'APHRODY_GPU_JSON {"compute_device_type": "OPTIX", "devices": []}\n'
        "trailing\n"
    )
    info = bpy_runner._parse_gpu_json(out)
    assert info["compute_device_type"] == "OPTIX"


def test_parse_gpu_json_missing_marker() -> None:
    with pytest.raises(BlenderRunnerError, match="marker"):
        bpy_runner._parse_gpu_json("no marker here\n")


def test_showcase_sprite_orchestration(tmp_path, monkeypatch) -> None:
    pytest.importorskip("PIL.Image")
    from aphrody import anim

    out = tmp_path / "show.webp"

    def fake_sprite(image, glb, **_kw):
        return bpy_runner.Path(glb)

    def fake_render(glb, frames_dir, **_kw):
        d = bpy_runner.Path(frames_dir)
        d.mkdir(parents=True, exist_ok=True)
        for i in range(3):
            (d / f"frame_{i:03d}.png").write_bytes(b"x")
        return {"compute_device_type": "OPTIX", "scene_device": "GPU"}

    monkeypatch.setattr(bpy_runner, "run_sprite_to_3d", fake_sprite)
    monkeypatch.setattr(bpy_runner, "render_turntable_gpu", fake_render)
    monkeypatch.setattr(
        anim,
        "build_animation",
        lambda frames, o, **_kw: bpy_runner.Path(o).write_bytes(
            b"RIFF\x00\x00\x00\x00WEBP"
        ),
    )

    info = bpy_runner.showcase_sprite("img.webp", out, frames=3)
    assert info["device"] == "OPTIX"
    assert info["rendered_frames"] == 3
    assert out.exists()
