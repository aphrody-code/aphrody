# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Module 5 — extract all exploitable source from the targets.

Pulls the real source out of the Antigravity install so it can be classified,
read and embedded:

* **asar archives** — ``app.asar`` / ``node_modules.asar`` are read with a
  from-scratch asar reader (Electron's asar = a Chromium ``Pickle`` header: a
  little-endian ``uint32`` payload size, a ``uint32`` JSON-header byte length,
  the JSON directory tree, then the concatenated file bodies). Text members
  (``.js`` / ``.ts`` / ``.json`` / ``.html`` / ``.css`` ...) are written out.
* **loose extension / config source** — ``.js`` / ``.json`` / ``.html`` under
  ``resources/app/extensions`` and config files are copied through.
* **Go sidecar artefacts** — the redress / GoReSym dumps already produced under
  ``var/data/antigravity-ide-re/redress`` (decompiled package list, recovered
  source, service methods) are linked into the corpus rather than re-running a
  133 MB decompile.

Everything lands under ``<out_dir>/extracted/`` with a manifest. The asar
reader is pure-stdlib (no Electron, no Node).
"""

from __future__ import annotations

import dataclasses
import json
import shutil
import struct
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable

#: Text member extensions worth extracting from an asar / tree as source.
SOURCE_EXTS = {
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "json",
    "jsonc",
    "html",
    "htm",
    "css",
    "map",
    "md",
    "txt",
    "yml",
    "yaml",
    "toml",
    "sh",
    "py",
    "proto",
    "pem",
}

#: Cap on a single extracted member's size (skip giant maps/blobs).
MAX_MEMBER_BYTES = 16 * 1024 * 1024

#: Where the prior RE pipeline left Go-sidecar artefacts (relative to repo).
REDRESS_REL = Path("var/data/antigravity-ide-re/redress")


@dataclasses.dataclass
class ExtractResult:
    """Summary of an extraction run.

    Attributes:
        out_dir: Root of the extracted corpus.
        asar_archives: ``[{archive, members_written, bytes}]`` per asar.
        loose_files: Count of loose source files copied.
        go_artifacts: Go-sidecar artefact files linked into the corpus.
        files: All extracted file paths (for the RAG indexer).
    """

    out_dir: str
    asar_archives: list[dict[str, Any]] = dataclasses.field(
        default_factory=list
    )
    loose_files: int = 0
    go_artifacts: list[str] = dataclasses.field(default_factory=list)
    files: list[str] = dataclasses.field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view."""
        d = dataclasses.asdict(self)
        d["total_files"] = len(self.files)
        return d


# -- asar reader ------------------------------------------------------------


def _read_pickle_uint32(buf: bytes, offset: int) -> tuple[int, int]:
    """Read a little-endian uint32 from ``buf`` at ``offset``."""
    (value,) = struct.unpack_from("<I", buf, offset)
    return value, offset + 4


def read_asar_header(data: bytes) -> tuple[dict[str, Any], int]:
    """Parse an asar header, returning ``(tree, body_offset)``.

    The asar layout is::

        uint32  pickle_payload_size  (= 4 + header_string_size, padded)
        uint32  header_string_size
        bytes   header_json [header_string_size]   (padded to 4 bytes)
        bytes   file bodies...

    Args:
        data: The asar archive bytes (at least the header).

    Returns:
        A ``(tree, body_offset)`` tuple where ``tree`` is the parsed JSON
        directory and ``body_offset`` is the byte offset of the first file body.

    Raises:
        ValueError: When the header cannot be parsed.
    """
    if len(data) < 16:
        raise ValueError("asar too small for a header")
    # Pickle: a uint32 payload size, then the payload. The payload begins with
    # a uint32 = header-string size.
    _payload_size, off = _read_pickle_uint32(data, 0)
    header_size, off = _read_pickle_uint32(data, off)
    json_start = off
    json_end = json_start + header_size
    if json_end > len(data):
        raise ValueError("asar header size exceeds file")
    try:
        tree = json.loads(data[json_start:json_end].decode("utf-8", "replace"))
    except ValueError as exc:
        raise ValueError(f"asar header is not valid JSON: {exc}") from exc
    # File bodies start after the JSON header, aligned up to 4 bytes.
    body_offset = (json_end + 3) & ~3
    return tree, body_offset


def _iter_asar_files(
    node: dict[str, Any], prefix: str = ""
) -> Iterable[tuple[str, dict[str, Any]]]:
    """Yield ``(relative_path, file_node)`` for every file in an asar tree."""
    for name, child in (node.get("files") or {}).items():
        rel = f"{prefix}/{name}" if prefix else name
        if "files" in child:
            yield from _iter_asar_files(child, rel)
        else:
            yield rel, child


def extract_asar(
    archive: str | Path,
    out_dir: str | Path,
    *,
    source_only: bool = True,
) -> dict[str, Any]:
    """Extract members from an asar archive into ``out_dir``.

    Args:
        archive: Path to an ``.asar`` file.
        out_dir: Destination directory (created; archive basename is appended).
        source_only: Only write members whose extension is in
            :data:`SOURCE_EXTS` (skips bundled binaries / images).

    Returns:
        A summary dict ``{archive, members_written, bytes, files}``.
    """
    archive = Path(archive)
    data = archive.read_bytes()
    tree, body_offset = read_asar_header(data)

    dest_root = Path(out_dir) / f"{archive.stem}.asar.extracted"
    written: list[str] = []
    total_bytes = 0

    for rel, fnode in _iter_asar_files(tree):
        ext = rel.rsplit(".", 1)[-1].lower() if "." in rel else ""
        if source_only and ext not in SOURCE_EXTS:
            continue
        size = int(fnode.get("size", 0))
        if size <= 0 or size > MAX_MEMBER_BYTES:
            continue
        # Unpacked members (size only, lives outside the archive) are skipped.
        if fnode.get("unpacked"):
            continue
        offset = int(fnode.get("offset", "-1"))
        start = body_offset + offset
        end = start + size
        if start < 0 or end > len(data):
            continue
        out_path = dest_root / rel
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_bytes(data[start:end])
        written.append(str(out_path))
        total_bytes += size

    return {
        "archive": str(archive),
        "members_written": len(written),
        "bytes": total_bytes,
        "files": written,
    }


# -- orchestration ----------------------------------------------------------


def _repo_root(start: Path | None = None) -> Path | None:
    """Find the repo root (nearest ancestor with a ``.git`` entry)."""
    here = (start or Path.cwd()).resolve()
    for d in (here, *here.parents):
        if (d / ".git").exists():
            return d
    return None


def link_go_artifacts(out_dir: str | Path) -> list[str]:
    """Copy the prior RE Go-sidecar artefacts into the corpus, if present.

    Reuses ``var/data/antigravity-ide-re/redress`` (recovered package list /
    source / service methods) rather than re-decompiling the 133 MB language
    server. Returns the list of copied artefact paths.
    """
    root = _repo_root()
    if root is None:
        return []
    redress = root / REDRESS_REL
    if not redress.is_dir():
        return []
    dest = Path(out_dir) / "go-sidecar"
    dest.mkdir(parents=True, exist_ok=True)
    copied: list[str] = []
    for src in redress.iterdir():
        if (
            src.is_file()
            and src.suffix in (".txt", ".json")
            and src.stat().st_size <= MAX_MEMBER_BYTES
        ):
            target = dest / src.name
            shutil.copy2(src, target)
            copied.append(str(target))
    return copied


def extract_all(
    entries: Iterable[Any],
    out_dir: str | Path,
    *,
    include_go: bool = True,
    max_loose: int = 20_000,
) -> ExtractResult:
    """Extract asar archives, loose source and Go artefacts from an inventory.

    Args:
        entries: Inventory entries (uses ``.path`` / ``.ext`` / ``.markers``).
        out_dir: Destination root; an ``extracted/`` subdir is created.
        include_go: Link the prior Go-sidecar RE artefacts into the corpus.
        max_loose: Cap on the number of loose source files copied.

    Returns:
        An :class:`ExtractResult`.
    """
    extracted = Path(out_dir) / "extracted"
    extracted.mkdir(parents=True, exist_ok=True)
    result = ExtractResult(out_dir=str(extracted))

    loose_dest = extracted / "loose"
    seen_asar: set[str] = set()

    for e in entries:
        if getattr(e, "is_dir", False):
            continue
        path = Path(e.path)
        ext = getattr(e, "ext", "")
        markers = getattr(e, "markers", [])

        if "asar" in markers or ext == "asar":
            if e.path in seen_asar:
                continue
            seen_asar.add(e.path)
            try:
                summary = extract_asar(path, extracted)
                result.asar_archives.append(
                    {k: v for k, v in summary.items() if k != "files"}
                )
                result.files.extend(summary["files"])
            except (OSError, ValueError, struct.error) as exc:
                result.asar_archives.append(
                    {"archive": e.path, "error": str(exc)}
                )
            continue

        # Loose source files (extension-driven), copied flat-ish under loose/.
        if ext in SOURCE_EXTS and result.loose_files < max_loose:
            if e.size <= 0 or e.size > MAX_MEMBER_BYTES:
                continue
            rel = getattr(e, "rel", path.name) or path.name
            out_path = loose_dest / rel
            try:
                out_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, out_path)
            except OSError:
                continue
            result.loose_files += 1
            result.files.append(str(out_path))

    if include_go:
        go = link_go_artifacts(extracted)
        result.go_artifacts = go
        result.files.extend(go)

    return result
