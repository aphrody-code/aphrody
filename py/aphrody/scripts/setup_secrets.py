# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Automated local secrets and environment configuration script."""

from __future__ import annotations

import sys
from pathlib import Path

# Add project root and package directory to sys.path for local imports
cwd = Path.cwd().resolve()
for directory in (cwd, *cwd.parents):
    if (directory / ".git").exists():
        sys.path.insert(0, str(directory))
        if (directory / "aphrody").exists():
            sys.path.insert(0, str(directory / "aphrody"))
        break

from aphrody.cli.setup import setup_secrets  # noqa: E402


def main() -> None:
    """Run automated credentials and secrets environment configuration."""
    success = setup_secrets()
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
