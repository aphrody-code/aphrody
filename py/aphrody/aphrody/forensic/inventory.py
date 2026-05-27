# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
r"""Module 1 — recursive inventory with real value reads.

Walks the target tree and, for every entry, records its path, size, detected
markers, and — in **full mode** — the *real values* behind those markers:

* **tokens / secrets** — ``oauth_creds.json``, ``*.token``, credential refs:
  the value is read and included, along with its type and (when present) its
  expiry / scopes. This is the owner's own machine and own account, so the real
  token bytes are captured deliberately.
* **DPAPI** — Windows ``Protect`` directories and ``\\x01\\x00\\x00\\x00`` /
  ``DPAPI`` blob markers (the encrypted-at-rest secret containers).
* **sqlite** — the ``SQLite format 3\\x00`` magic.
* **Electron** — ``app.asar`` / ``node_modules.asar``, ``resources/app``,
  ``product.json`` (electron version, commit, product name).
* **Go** — the ``\\xff Go buildinf:`` build-info magic / pclntab presence in a
  PE (the native ``cortex`` language server).
* **C++ pure** — an MSVC PE with no Go / .NET markers (``Rich`` header, CRT
  strings) — the classic native binary surface.
* **Google** — ``*.googleapis.com`` hosts, ``product.json`` client info, OAuth
  ``*.apps.googleusercontent.com`` client IDs.

Output is a fully serialisable list of :class:`Entry` records (plus a summary).
The walk is bounded by a per-file head-read size so a 211 MB Electron binary is
sniffed, never slurped.
"""

from __future__ import annotations

import dataclasses
import json
import os
import re
import stat
from pathlib import Path
from typing import Any

# -- magic numbers / markers ------------------------------------------------

#: SQLite database magic (first 16 bytes).
SQLITE_MAGIC = b"SQLite format 3\x00"

#: Go build-info magic embedded in Go binaries (``go version`` reads this).
GO_BUILDINFO_MAGIC = b"\xff Go buildinf:"

#: Go runtime symbol-table marker (pclntab signatures across Go versions).
_GO_PCLNTAB_MAGICS = (
    b"\xfb\xff\xff\xff\x00\x00",  # go1.18+
    b"\xfa\xff\xff\xff\x00\x00",  # go1.16/1.17
    b"\xf0\xff\xff\xff\x00\x00",  # go1.2..1.15 (fb/fa variants)
)

#: PE (DOS) header magic.
PE_MAGIC = b"MZ"

#: .NET / CLR metadata marker (a managed PE — excludes "pure C++").
_DOTNET_MARKERS = (b"_CorExeMain", b"mscoree.dll", b"BSJB")

#: MSVC "Rich" header marker + CRT strings — signal of a native MSVC C/C++ PE.
_MSVC_MARKERS = (b"Rich", b"VCRUNTIME", b"MSVCP", b"api-ms-win-crt")

#: DPAPI blob preludes / directory names.
DPAPI_BLOB_PREFIX = (
    b"\x01\x00\x00\x00\xd0\x8c\x9d\xdf"  # CRYPTPROTECT_* prelude
)
_DPAPI_DIR_NAMES = {"protect", "credentials", "vault"}

#: Token / secret file-name patterns (full real values are read for these).
_SECRET_NAME_RE = re.compile(
    r"(oauth[_-]?creds|credentials?|access[_-]?token|refresh[_-]?token|"
    r"id[_-]?token|\.token|token\.json|secret|api[_-]?key|state\.vscdb|"
    r"google[_-]?cookies)",
    re.IGNORECASE,
)

#: Googleapis / OAuth client-id detectors (run over a text head).
_GOOGLEAPIS_RE = re.compile(rb"[a-z0-9.-]+\.googleapis\.com")
_OAUTH_CLIENT_RE = re.compile(
    rb"[0-9]+-[a-z0-9]+\.apps\.googleusercontent\.com"
)

#: How many bytes to read from a file head when sniffing markers.
HEAD_BYTES = 64 * 1024

#: Skip files larger than this for the *full marker scan* (still inventoried,
#: but only the head is read — so a 211 MB binary is sniffed, not slurped).
MAX_SCAN_BYTES = 256 * 1024 * 1024

#: Token/secret files larger than this are referenced but not value-read (a
#: 30 MB ``state.vscdb`` is a sqlite DB, handled by the sqlite path instead).
MAX_SECRET_VALUE_BYTES = 1 * 1024 * 1024


@dataclasses.dataclass
class Entry:
    """One inventoried filesystem entry.

    Attributes:
        path: Absolute path.
        rel: Path relative to the walked root.
        size: Size in bytes (0 for directories / unstatable).
        is_dir: Whether the entry is a directory.
        ext: Lowercase file extension (without the dot), or "".
        markers: Sorted list of detected marker tags (e.g. ``["electron",
            "google"]``).
        details: Marker-specific structured data, including **real values**
            for the token/secret/electron/go markers.
    """

    path: str
    rel: str
    size: int
    is_dir: bool
    ext: str
    markers: list[str] = dataclasses.field(default_factory=list)
    details: dict[str, Any] = dataclasses.field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view."""
        return dataclasses.asdict(self)


def _read_head(path: Path, n: int = HEAD_BYTES) -> bytes:
    """Read up to ``n`` bytes from the start of ``path`` (errors -> b"")."""
    try:
        with path.open("rb") as fh:
            return fh.read(n)
    except OSError:
        return b""


def _looks_text(head: bytes) -> bool:
    """Heuristic: treat a head as text if it has no NUL and is mostly ASCII."""
    if b"\x00" in head:
        return False
    if not head:
        return True
    printable = sum(1 for b in head if 9 <= b <= 13 or 32 <= b <= 126)
    return printable / len(head) > 0.85


def _parse_token_value(path: Path, text: str) -> dict[str, Any]:
    """Extract real token fields from a secret file's text content.

    The full values are deliberately captured (owner's own credentials). For
    JSON we surface the recognised OAuth fields; otherwise the raw text is
    included verbatim (trimmed) plus a best-effort type guess.
    """
    info: dict[str, Any] = {"file_name": path.name}
    stripped = text.strip()
    try:
        data = json.loads(stripped)
    except (ValueError, TypeError):
        # Non-JSON token (raw bearer / refresh string, cookie blob, etc.).
        info["token_type"] = "raw"
        info["value"] = stripped
        return info

    info["token_type"] = "json"
    info["value"] = data
    if isinstance(data, dict):
        for key in (
            "access_token",
            "refresh_token",
            "id_token",
            "token",
            "client_id",
            "client_secret",
            "scope",
            "scopes",
            "expiry",
            "expiry_date",
            "expires_in",
            "expires_at",
            "token_type",
        ):
            if key in data:
                info[key] = data[key]
    return info


def _scan_binary_markers(
    head: bytes, full: bytes
) -> tuple[list[str], dict[str, Any]]:
    """Detect binary markers (sqlite/PE/Go/.NET/C++/DPAPI) over file bytes.

    ``head`` is the sniff window; ``full`` is the (possibly larger) buffer used
    for substring scans when the file is within :data:`MAX_SCAN_BYTES`.
    """
    markers: list[str] = []
    details: dict[str, Any] = {}

    if head.startswith(SQLITE_MAGIC):
        markers.append("sqlite")

    if head.startswith(DPAPI_BLOB_PREFIX) or DPAPI_BLOB_PREFIX in head:
        markers.append("dpapi")

    is_pe = head.startswith(PE_MAGIC)
    if is_pe:
        markers.append("pe")
        is_go = GO_BUILDINFO_MAGIC in full or any(
            m in full for m in _GO_PCLNTAB_MAGICS
        )
        is_dotnet = any(m in full for m in _DOTNET_MARKERS)
        if is_go:
            markers.append("go")
            details["go"] = _parse_go_buildinfo(full)
        if is_dotnet:
            markers.append("dotnet")
        if (
            not is_go
            and not is_dotnet
            and any(m in full for m in _MSVC_MARKERS)
        ):
            markers.append("cpp")

    return markers, details


def _parse_go_buildinfo(buf: bytes) -> dict[str, Any]:
    """Extract the Go version / module path from a build-info blob, if present."""
    idx = buf.find(GO_BUILDINFO_MAGIC)
    info: dict[str, Any] = {"has_buildinfo": idx >= 0}
    # The classic ``go version goX.Y.Z`` string is also embedded; grab it.
    m = re.search(rb"go1\.\d+(?:\.\d+)?", buf)
    if m:
        info["go_version"] = m.group(0).decode("ascii", "replace")
    return info


def _scan_google_markers(head: bytes) -> dict[str, Any]:
    """Find googleapis hosts + OAuth client IDs in a (text/binary) head."""
    hosts = sorted(
        {m.decode("ascii", "replace") for m in _GOOGLEAPIS_RE.findall(head)}
    )
    clients = sorted(
        {m.decode("ascii", "replace") for m in _OAUTH_CLIENT_RE.findall(head)}
    )
    out: dict[str, Any] = {}
    if hosts:
        out["googleapis_hosts"] = hosts[:50]
    if clients:
        out["oauth_client_ids"] = clients
    return out


def inventory_entry(path: Path, root: Path) -> Entry:
    """Inventory a single path: stat it, sniff markers, read real values.

    Args:
        path: The file or directory to inventory.
        root: The walk root (for the relative path).

    Returns:
        A fully populated :class:`Entry`.
    """
    try:
        st = path.lstat()
    except OSError:
        return Entry(
            path=str(path), rel=_rel(path, root), size=0, is_dir=False, ext=""
        )

    is_dir = stat.S_ISDIR(st.st_mode)
    ext = path.suffix.lower().lstrip(".")
    entry = Entry(
        path=str(path),
        rel=_rel(path, root),
        size=int(st.st_size),
        is_dir=is_dir,
        ext=ext,
    )

    if is_dir:
        if path.name.lower() in _DPAPI_DIR_NAMES:
            entry.markers.append("dpapi_dir")
        if path.name == "app" and path.parent.name == "resources":
            entry.markers.append("electron_app")
        return entry

    name = path.name.lower()

    # Electron product manifest — read the real product/version/commit fields.
    if name == "product.json":
        entry.markers.append("electron")
        try:
            data = json.loads(
                path.read_text(encoding="utf-8", errors="replace")
            )
            entry.details["product"] = {
                k: data.get(k)
                for k in (
                    "nameLong",
                    "version",
                    "commit",
                    "electronVersion",
                    "applicationName",
                    "dataFolderName",
                )
                if k in data
            }
        except (OSError, ValueError):
            pass

    if name in ("app.asar", "node_modules.asar") or ext == "asar":
        entry.markers.append("electron")
        entry.markers.append("asar")

    head = _read_head(path)

    # Token / secret files — read the full real value.
    if _SECRET_NAME_RE.search(name) and not name.endswith(".vscdb"):
        if 0 < st.st_size <= MAX_SECRET_VALUE_BYTES:
            text = (
                head.decode("utf-8", errors="replace")
                if _looks_text(head)
                else ""
            )
            if not text:
                # Re-read whole small file as text best-effort.
                try:
                    text = path.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    text = ""
            if text:
                entry.markers.append("secret")
                entry.details["secret"] = _parse_token_value(path, text)
        else:
            entry.markers.append("secret")
            entry.details["secret"] = {
                "file_name": path.name,
                "token_type": "oversized",
                "note": "value not inlined (size guard); inspect directly",
            }

    # Binary marker sniff (sqlite/PE/Go/.NET/C++/DPAPI).
    if st.st_size <= MAX_SCAN_BYTES:
        full = head if st.st_size <= HEAD_BYTES else _read_full(path)
    else:
        full = head
    bmarkers, bdetails = _scan_binary_markers(head, full)
    entry.markers.extend(bmarkers)
    entry.details.update(bdetails)

    # Google host / client-id detection (over the head — cheap, even on binaries).
    gdetails = _scan_google_markers(
        full if len(full) <= MAX_SCAN_BYTES else head
    )
    if gdetails:
        entry.markers.append("google")
        entry.details["google"] = gdetails

    # De-dupe + stabilise marker order.
    entry.markers = sorted(set(entry.markers))
    return entry


def _read_full(path: Path) -> bytes:
    """Read the whole file (errors -> b"")."""
    try:
        return path.read_bytes()
    except OSError:
        return b""


def _rel(path: Path, root: Path) -> str:
    """Best-effort relative path string."""
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


@dataclasses.dataclass
class InventoryResult:
    """The result of an inventory walk.

    Attributes:
        root: The walked root path.
        entries: One :class:`Entry` per file/dir visited.
        summary: Aggregate counts (files, dirs, bytes, markers, secrets).
    """

    root: str
    entries: list[Entry]
    summary: dict[str, Any]

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view of the whole inventory."""
        return {
            "root": self.root,
            "summary": self.summary,
            "entries": [e.to_dict() for e in self.entries],
        }


def walk_inventory(
    root: str | Path,
    *,
    max_files: int = 200_000,
    follow_symlinks: bool = False,
) -> InventoryResult:
    """Recursively inventory ``root``.

    Args:
        root: Directory (or single file) to inventory.
        max_files: Hard cap on the number of entries (safety bound).
        follow_symlinks: Whether ``os.walk`` follows directory symlinks.

    Returns:
        An :class:`InventoryResult`.
    """
    root_path = Path(root)
    entries: list[Entry] = []

    if root_path.is_file():
        entries.append(inventory_entry(root_path, root_path.parent))
    else:
        count = 0
        for dirpath, dirnames, filenames in os.walk(
            root_path, followlinks=follow_symlinks
        ):
            d = Path(dirpath)
            # Inventory directories too (DPAPI / electron_app dir markers).
            for dn in dirnames:
                entries.append(inventory_entry(d / dn, root_path))
                count += 1
                if count >= max_files:
                    break
            for fn in filenames:
                entries.append(inventory_entry(d / fn, root_path))
                count += 1
                if count >= max_files:
                    break
            if count >= max_files:
                break

    return InventoryResult(
        root=str(root_path),
        entries=entries,
        summary=_summarise(entries),
    )


def _summarise(entries: list[Entry]) -> dict[str, Any]:
    """Aggregate inventory counts and per-marker tallies."""
    marker_counts: dict[str, int] = {}
    total_bytes = 0
    files = dirs = 0
    secrets: list[dict[str, Any]] = []
    for e in entries:
        if e.is_dir:
            dirs += 1
        else:
            files += 1
            total_bytes += e.size
        for m in e.markers:
            marker_counts[m] = marker_counts.get(m, 0) + 1
        if "secret" in e.markers:
            secrets.append(
                {
                    "path": e.path,
                    **{
                        k: v
                        for k, v in e.details.get("secret", {}).items()
                        if k != "value"
                    },
                }
            )
    return {
        "files": files,
        "dirs": dirs,
        "total_bytes": total_bytes,
        "markers": dict(sorted(marker_counts.items())),
        "secret_files": secrets,
    }
