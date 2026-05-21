# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.sprite_anim` (mocked — no network)."""

from __future__ import annotations

from pathlib import Path

from aphrody import sprite_anim


def test_actions_catalogue() -> None:
    expected = {"walk", "run", "jump", "crouch", "fly", "kick_ball"}
    assert set(sprite_anim.ACTIONS) == expected
    assert all(len(phases) == 3 for phases in sprite_anim.ACTIONS.values())


def test_build_action_prompt() -> None:
    p = sprite_anim.build_action_prompt("kicking a soccer ball")
    assert "olive-green long flowing hair" in p  # identity preserved
    assert "kicking a soccer ball" in p
    assert "white background" in p


class _FakeNB:
    def __init__(self, **_kw) -> None:
        pass

    def edit_image(self, _base, _prompt, *, out, image_size):
        Path(out).write_bytes(b"\x89PNG\r\n")
        return Path(out)


def test_generate_action_frames(tmp_path, monkeypatch) -> None:
    from aphrody import images

    monkeypatch.setattr(images, "NanoBanana", _FakeNB)
    frames = sprite_anim.generate_action_frames("base.webp", "run", tmp_path)
    assert [f.name for f in frames] == [
        "run_r0.png",
        "run_r1.png",
        "run_r2.png",
    ]
    assert all(f.exists() for f in frames)


def test_generate_actions_manifest(tmp_path, monkeypatch) -> None:
    from aphrody import anim, images

    monkeypatch.setattr(images, "NanoBanana", _FakeNB)
    monkeypatch.setattr(
        anim,
        "build_animation",
        lambda frames, o, **_kw: Path(o).write_bytes(b"RIFF"),
    )
    manifest = sprite_anim.generate_actions(
        "base.webp", tmp_path, actions=["run", "jump"]
    )
    assert set(manifest) == {"run", "jump", "showreel"}
    assert len(manifest["run"]["frames"]) == 3
    assert (tmp_path / "run.webp").exists()
    assert (tmp_path / "showreel.webp").exists()
