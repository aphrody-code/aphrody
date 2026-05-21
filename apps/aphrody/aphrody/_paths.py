# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Resolve where aphrody keeps its private secrets (tokens, cookies).

aphrody never embeds secrets; it reads them at runtime from a private,
git-ignored directory. The directory is resolved in this order:

1. ``$APHRODY_SECRETS_DIR`` — an explicit override.
2. ``<repo root>/var/secrets`` — when running inside the aphrody repository
   (the repo root is the nearest ancestor of the CWD that contains a ``.git``
   entry). This matches the repo convention of keeping git-ignored secrets
   under ``var/`` and is the cross-platform token home on Linux (cible #1),
   where there is no Windows Credential Manager.
3. ``~/.aphrody`` — fallback for a standalone install outside the repo.

Resolution is dynamic (per call) so a process that changes directory, or a
test that sets ``$APHRODY_SECRETS_DIR``, always sees the right location.
"""

from __future__ import annotations

import os
from pathlib import Path

_REPO_MARKER = ".git"


def _repo_var_secrets() -> Path | None:
    """Return ``<repo root>/var/secrets`` if the CWD is inside the repo."""
    here = Path.cwd().resolve()
    for directory in (here, *here.parents):
        if (directory / _REPO_MARKER).exists():
            return directory / "var" / "secrets"
    return None


def secrets_dir() -> Path:
    """Return the directory aphrody uses for private secrets.

    See the module docstring for the resolution order.

    Returns:
        The resolved secrets directory (it may not yet exist).
    """
    override = os.environ.get("APHRODY_SECRETS_DIR")
    if override:
        return Path(override).expanduser()
    repo = _repo_var_secrets()
    if repo is not None:
        return repo
    return Path.home() / ".aphrody"


def secret_file(name: str) -> Path:
    """Return the path of secret file *name* inside :func:`secrets_dir`.

    Args:
        name: The file name (e.g. ``"google-cookies.json"``).

    Returns:
        The absolute path to the secret file.
    """
    return secrets_dir() / name
