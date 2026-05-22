# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Well-known Antigravity IDE targets and target-name resolution.

The forensic pipeline accepts either a literal filesystem path or one of a few
short names that expand to the canonical Antigravity install / data
directories (see ``docs/research/antigravity-ide-re.md``). Resolution is
environment-aware so the same names work on the owner's Windows box and degrade
gracefully elsewhere (Linux cible #1 has no ``%LOCALAPPDATA%``).
"""

from __future__ import annotations

import os
from pathlib import Path

#: Short name -> a function producing its absolute path (or ``None`` if the
#: host has no such location). Kept lazy so importing this module never touches
#: the environment in a way that surprises tests.
_TARGETS: dict[str, str] = {
    # The installed program directory (binaries, Electron, the Go LS).
    "install": r"%LOCALAPPDATA%\Programs\Antigravity IDE",
    # Roaming app data (settings, state, logs).
    "appdata": r"%APPDATA%\Antigravity IDE",
    # The dotted variant some builds use.
    "dotdir": r"%APPDATA%\.antigravity-ide",
    # The agent's local cache / credentials (``agy``).
    "agy": r"%LOCALAPPDATA%\agy",
    # The Gemini CLI / sidecar home (~/.gemini).
    "gemini": r"~\.gemini",
}


def _expand(raw: str) -> Path:
    """Expand environment variables and ``~`` in a raw target template."""
    return Path(os.path.expandvars(os.path.expanduser(raw)))


def known_targets() -> dict[str, str]:
    """Return the resolved (expanded) path string for every known target name."""
    return {name: str(_expand(tpl)) for name, tpl in _TARGETS.items()}


def resolve_target(target: str) -> Path:
    """Resolve a target name or literal path to an absolute :class:`Path`.

    Args:
        target: A short name (``install`` / ``appdata`` / ``dotdir`` / ``agy``
            / ``gemini``) or any filesystem path.

    Returns:
        The resolved absolute path (it may not exist).
    """
    if target in _TARGETS:
        return _expand(_TARGETS[target])
    return _expand(target)


def default_targets() -> list[Path]:
    """Return every known Antigravity target path that currently exists."""
    out: list[Path] = []
    for tpl in _TARGETS.values():
        path = _expand(tpl)
        if path.exists():
            out.append(path)
    return out
