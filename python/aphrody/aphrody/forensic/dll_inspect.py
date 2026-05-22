# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Module 3 — static PE / DLL inspection with LIEF.

For every PE (``.exe`` / ``.dll`` / ``.node`` / language-server binary), LIEF
(Apache-2.0) parses the on-disk structure **without executing it** — that's the
RE meaning of "test the DLLs": inspect the import surface (which DLLs and APIs
the binary calls), the export surface (what it offers), the section layout, and
the Authenticode signature (who signed it). No analysed binary is ever run.

A LIEF instance is not reused (LIEF parses per file); the parser is imported
lazily and injected in tests via the ``lief_mod=`` parameter.
"""

from __future__ import annotations

import dataclasses
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable

#: Extensions worth handing to LIEF as PE candidates.
PE_EXTENSIONS = {"exe", "dll", "node", "sys", "ocx", "cpl", "scr"}


@dataclasses.dataclass
class PeReport:
    """A static PE inspection report.

    Attributes:
        path: The inspected file path.
        machine: Target machine (e.g. ``AMD64``).
        is_dll: Whether the PE is a DLL.
        subsystem: PE subsystem (``GUI`` / ``CONSOLE`` / ...).
        imports: ``{dll: [function, ...]}`` import map (entries truncated).
        import_dlls: The set of imported DLL names (full).
        exports: Exported symbol names (truncated).
        sections: ``[{name, vsize, rawsize, entropy, characteristics}]``.
        signed: Whether an Authenticode signature is present.
        signers: Subject names from the signature, when verifiable.
        error: A parse error message, when LIEF could not read the file.
    """

    path: str
    machine: str = ""
    is_dll: bool = False
    subsystem: str = ""
    imports: dict[str, list[str]] = dataclasses.field(default_factory=dict)
    import_dlls: list[str] = dataclasses.field(default_factory=list)
    exports: list[str] = dataclasses.field(default_factory=list)
    sections: list[dict[str, Any]] = dataclasses.field(default_factory=list)
    signed: bool = False
    signers: list[str] = dataclasses.field(default_factory=list)
    error: str | None = None

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view."""
        return dataclasses.asdict(self)


def _lief():
    """Import LIEF lazily (kept out of module import for fast CLI startup)."""
    import lief

    return lief


def is_pe_candidate(path: str | Path, ext: str | None = None) -> bool:
    """Return whether ``path`` looks like a PE worth handing to LIEF."""
    e = (ext if ext is not None else Path(path).suffix.lstrip(".")).lower()
    return e in PE_EXTENSIONS


def inspect_pe(
    path: str | Path,
    *,
    lief_mod: Any | None = None,
    max_imports_per_dll: int = 64,
    max_exports: int = 256,
) -> PeReport:
    """Statically inspect a PE/DLL with LIEF.

    Args:
        path: The PE file to inspect (never executed).
        lief_mod: An injected LIEF-like module (tests pass a fake).
        max_imports_per_dll: Cap on functions listed per imported DLL.
        max_exports: Cap on exported symbols listed.

    Returns:
        A :class:`PeReport`. On parse failure ``error`` is set and the rest is
        left at defaults.
    """
    path = str(path)
    report = PeReport(path=path)
    lief = lief_mod if lief_mod is not None else _lief()

    try:
        binary = lief.parse(path)
    except Exception as exc:
        report.error = f"lief.parse failed: {exc}"
        return report

    if binary is None:
        report.error = "not a parseable PE (lief.parse returned None)"
        return report

    header = getattr(binary, "header", None)
    if header is not None:
        report.machine = _enum_name(getattr(header, "machine", ""))
        chars = getattr(header, "characteristics_list", None) or []
        report.is_dll = any("DLL" in _enum_name(c) for c in chars)

    opt = getattr(binary, "optional_header", None)
    if opt is not None:
        report.subsystem = _enum_name(getattr(opt, "subsystem", ""))

    # Imports — the dependency surface.
    for imp in getattr(binary, "imports", []) or []:
        dll = getattr(imp, "name", "") or ""
        funcs: list[str] = []
        for entry in getattr(imp, "entries", []) or []:
            fname = getattr(entry, "name", None)
            if fname:
                funcs.append(fname)
            elif getattr(entry, "is_ordinal", False):
                funcs.append(f"#ordinal:{getattr(entry, 'ordinal', '?')}")
            if len(funcs) >= max_imports_per_dll:
                break
        if dll:
            report.imports[dll] = funcs
    report.import_dlls = sorted(report.imports)

    # Exports — what the binary offers.
    exported = getattr(binary, "get_export", None)
    exp_obj = (
        exported() if callable(exported) else getattr(binary, "export", None)
    )
    if exp_obj is not None:
        for sym in getattr(exp_obj, "entries", []) or []:
            name = getattr(sym, "name", None)
            if name:
                report.exports.append(name)
            if len(report.exports) >= max_exports:
                break

    # Sections — layout + entropy (entropy flags packed/encrypted sections).
    for sec in getattr(binary, "sections", []) or []:
        report.sections.append(
            {
                "name": getattr(sec, "name", ""),
                "virtual_size": int(getattr(sec, "virtual_size", 0) or 0),
                "raw_size": int(getattr(sec, "size", 0) or 0),
                "entropy": round(float(getattr(sec, "entropy", 0.0) or 0.0), 3),
            }
        )

    # Authenticode signature surface.
    has_sig = getattr(binary, "has_signatures", None)
    report.signed = bool(has_sig() if callable(has_sig) else has_sig)
    if not report.signed:
        report.signed = bool(getattr(binary, "signatures", None))
    for sig in getattr(binary, "signatures", []) or []:
        for signer in getattr(sig, "signers", []) or []:
            cert = getattr(signer, "cert", None)
            subj = getattr(cert, "subject", None) if cert is not None else None
            if subj:
                report.signers.append(str(subj))

    return report


def _enum_name(value: Any) -> str:
    """Stringify a LIEF enum / value to a short name."""
    if value is None:
        return ""
    name = getattr(value, "name", None)
    if name:
        return str(name)
    text = str(value)
    return text.rsplit(".", 1)[-1] if "." in text else text


def inspect_entries(
    entries: Iterable[Any],
    *,
    lief_mod: Any | None = None,
    limit: int | None = None,
) -> list[PeReport]:
    """Inspect every PE-candidate entry in an inventory.

    Args:
        entries: Inventory entries (uses ``.path``/``.ext``/``.markers``).
        lief_mod: Injected LIEF module (tests pass a fake).
        limit: Optional cap on the number of PEs inspected.

    Returns:
        One :class:`PeReport` per inspected PE.
    """
    out: list[PeReport] = []
    for e in entries:
        if getattr(e, "is_dir", False):
            continue
        ext = getattr(e, "ext", "")
        markers = getattr(e, "markers", [])
        if not (is_pe_candidate(e.path, ext) or "pe" in markers):
            continue
        out.append(inspect_pe(e.path, lief_mod=lief_mod))
        if limit is not None and len(out) >= limit:
            break
    return out
