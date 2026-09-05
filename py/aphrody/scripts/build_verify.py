# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Automated package building, code style linting, formatting and test suite verification."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


def run_command(args: list[str], cwd: Path) -> tuple[int, str, str]:
    """Run a process command and return returncode, stdout and stderr."""
    res = subprocess.run(
        args, capture_output=True, text=True, check=False, cwd=str(cwd)
    )
    return res.returncode, res.stdout.strip(), res.stderr.strip()


def main() -> None:
    """Run automated verification checks and builds the package wheel."""
    print("=== Aphrody Build & Verify Pipeline ===")

    # 1. Resolve repository root
    cwd = Path.cwd().resolve()
    repo_root = None
    for directory in (cwd, *cwd.parents):
        if (directory / ".git").exists():
            repo_root = directory
            break

    if not repo_root:
        print("Error: Could not locate repository root (.git).")
        sys.exit(1)

    print(f"Repository Root: {repo_root}")

    # 2. Check if uv is installed
    uv_path = shutil.which("uv")
    if not uv_path:
        print("Error: 'uv' package manager not found. Please install uv first.")
        sys.exit(1)
    print(f"Found uv at: {uv_path}")

    # 3. Code formatting & linting check (Ruff)
    print("\n--- [Stage 1/4] Running Linter & Formatter ---")
    code, out, err = run_command(["uv", "run", "ruff", "check"], repo_root)
    if code != 0:
        print("FAIL: ruff check failed!")
        if out:
            print(out)
        if err:
            print(err)
        sys.exit(1)
    print("PASS: Ruff lint checks passed.")

    code, out, err = run_command(
        ["uv", "run", "ruff", "format", "--check"], repo_root
    )
    if code != 0:
        print("FAIL: ruff format check failed!")
        if out:
            print(out)
        if err:
            print(err)
        sys.exit(1)
    print("PASS: Ruff formatting checks passed.")

    # 4. Local unit test execution (pytest)
    print("\n--- [Stage 2/4] Running Test Suite (pytest) ---")
    # Using local pytest excluding live API calls
    code, out, err = run_command(
        ["uv", "run", "pytest", "-m", "not live_api"], repo_root
    )
    if code != 0:
        print("FAIL: pytest suite failed!")
        if out:
            # Print last 30 lines of stdout to avoid cluttering but show failures
            lines = out.splitlines()
            print("\n".join(lines[-30:]))
        if err:
            print(err)
        sys.exit(1)
    print("PASS: Pytest suite passed successfully.")

    # 5. Build the wheel package
    print("\n--- [Stage 3/4] Building Wheel Package ---")
    # Clean previous build artifacts first
    dist_dir = repo_root / "dist"
    if dist_dir.exists():
        shutil.rmtree(dist_dir)

    code, out, err = run_command(
        ["uv", "build", "--package", "aphrody", "--wheel"], repo_root
    )
    if code != 0:
        # Fallback to python -m build if workspace build needs it
        print(
            "Note: uv build failed or not configured for package, trying fallback build..."
        )
        # Check if python build is available
        code, out, err = run_command(
            [
                "uv",
                "run",
                "python",
                "-m",
                "build",
                "--wheel",
                "--outdir",
                "dist",
            ],
            repo_root / "aphrody",
        )
        if code != 0:
            print("FAIL: Packaging build failed!")
            if out:
                print(out)
            if err:
                print(err)
            sys.exit(1)

    print("PASS: Package wheel built successfully.")

    # 6. Verify built files
    print("\n--- [Stage 4/4] Verifying Package Wheel Artifacts ---")
    dist_dir = repo_root / "dist"
    if not dist_dir.exists():
        # check inside aphrody/dist
        dist_dir = repo_root / "aphrody" / "dist"

    if not dist_dir.exists():
        print("FAIL: dist/ directory not found after build!")
        sys.exit(1)

    wheels = list(dist_dir.glob("aphrody-*.whl"))
    if not wheels:
        print("FAIL: No aphrody-*.whl package found in dist/!")
        sys.exit(1)

    for wheel in wheels:
        size = wheel.stat().st_size
        print(f"Artifact: {wheel.name} ({size} bytes)")
        if size < 5000:
            print(
                f"FAIL: Built wheel size is suspiciously small: {size} bytes."
            )
            sys.exit(1)

    print("PASS: Built artifacts verified.")
    print("\n=========================================")
    print("Build & Verify pipeline completed successfully!")
    print("=========================================")


if __name__ == "__main__":
    main()
