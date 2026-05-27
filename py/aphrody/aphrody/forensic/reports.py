# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Module 4 — document conversion (markitdown) + consolidated report.

Two jobs:

* :func:`convert_documents` runs Microsoft's markitdown (MIT) over every
  inventoried document (pdf / html / docx / pptx / xlsx ...), turning them into
  markdown that joins the RAG corpus.
* :func:`build_markdown_report` assembles the consolidated forensic report from
  the inventory, classification, PE inspection, extraction and (optional) LLM
  synthesis — a single human-readable markdown file.

markitdown is imported lazily and injected in tests via ``md=``.
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable

#: Extensions markitdown handles well (the document corpus).
DOC_EXTS = {
    "pdf",
    "docx",
    "pptx",
    "xlsx",
    "html",
    "htm",
    "csv",
    "epub",
    "rtf",
}


def _markitdown(md: Any | None = None) -> Any:
    """Return a reusable MarkItDown instance (lazy import)."""
    if md is not None:
        return md
    from markitdown import MarkItDown

    return MarkItDown()


def convert_document(
    path: str | Path, *, md: Any | None = None
) -> dict[str, Any]:
    """Convert one document to markdown via markitdown.

    Args:
        path: Document path.
        md: An injected MarkItDown-like instance (tests pass a fake).

    Returns:
        ``{path, markdown}`` on success, or ``{path, error}`` on failure.
    """
    converter = _markitdown(md)
    try:
        result = converter.convert(str(path))
        text = getattr(result, "text_content", None)
        if text is None:
            text = getattr(result, "markdown", "") or str(result)
        return {"path": str(path), "markdown": text}
    except Exception as exc:
        return {"path": str(path), "error": str(exc)}


def convert_documents(
    entries: Iterable[Any],
    out_dir: str | Path,
    *,
    md: Any | None = None,
    limit: int | None = None,
) -> list[dict[str, Any]]:
    """Convert every document entry to markdown, writing ``.md`` siblings.

    Args:
        entries: Inventory entries (uses ``.path`` / ``.ext``).
        out_dir: Destination for the converted ``.md`` files (``docs/`` subdir).
        md: An injected MarkItDown-like instance.
        limit: Optional cap on conversions.

    Returns:
        One result dict per converted document (with the written ``md_path``).
    """
    converter = _markitdown(md)
    docs_dir = Path(out_dir) / "docs"
    out: list[dict[str, Any]] = []
    for e in entries:
        if getattr(e, "is_dir", False):
            continue
        ext = getattr(e, "ext", "")
        if ext not in DOC_EXTS:
            continue
        res = convert_document(e.path, md=converter)
        if res.get("markdown"):
            docs_dir.mkdir(parents=True, exist_ok=True)
            md_path = docs_dir / (Path(e.path).name + ".md")
            try:
                md_path.write_text(res["markdown"], encoding="utf-8")
                res["md_path"] = str(md_path)
            except OSError as exc:
                res["error"] = str(exc)
        out.append(res)
        if limit is not None and len(out) >= limit:
            break
    return out


def build_markdown_report(
    *,
    target: str,
    inventory: dict[str, Any],
    classification: dict[str, Any],
    pe_reports: list[dict[str, Any]],
    extraction: dict[str, Any] | None = None,
    documents: list[dict[str, Any]] | None = None,
    llm: dict[str, Any] | None = None,
) -> str:
    """Assemble the consolidated forensic report as markdown.

    Args:
        target: The analysed target (name or path).
        inventory: The inventory summary dict.
        classification: The classification aggregate dict.
        pe_reports: PE inspection reports (as dicts).
        extraction: Extraction summary (optional).
        documents: Converted-document results (optional).
        llm: LLM synthesis result (optional).

    Returns:
        The full markdown report as a string.
    """
    inv = inventory.get("summary", inventory)
    lines: list[str] = []
    a = lines.append

    a(f"# Forensic report — `{target}`")
    a("")
    a(
        "Full-mode forensic + classification + RAG + auto-ML pass over the "
        "Antigravity IDE surface. Owner's own machine and account; static "
        "analysis only (no analysed binary is executed)."
    )
    a("")

    a("## 1. Inventory")
    a("")
    a(
        f"- Files: **{inv.get('files', 0)}**, dirs: {inv.get('dirs', 0)}, "
        f"total bytes: {inv.get('total_bytes', 0):,}"
    )
    markers = inv.get("markers", {})
    if markers:
        a("- Markers:")
        for name, count in markers.items():
            a(f"  - `{name}`: {count}")
    secrets = inv.get("secret_files", [])
    a("")
    a(f"## 2. Secrets / tokens ({len(secrets)})")
    a("")
    if secrets:
        for s in secrets:
            kind = s.get("token_type", "?")
            extra = []
            for k in ("scope", "scopes", "expiry", "expiry_date", "expires_in"):
                if k in s:
                    extra.append(f"{k}={s[k]}")
            suffix = (" — " + ", ".join(str(x) for x in extra)) if extra else ""
            a(f"- `{s.get('path')}` ({kind}){suffix}")
    else:
        a("_None detected in this subtree._")

    a("")
    a("## 3. Classification (Magika)")
    a("")
    by_cat = classification.get("by_category", {})
    if by_cat:
        a("| category | count |")
        a("|----------|------:|")
        for cat, count in by_cat.items():
            a(f"| {cat} | {count} |")
    a("")

    a(f"## 4. PE / DLL inspection — LIEF ({len(pe_reports)})")
    a("")
    for pe in pe_reports[:50]:
        if pe.get("error"):
            a(f"- `{pe['path']}` — error: {pe['error']}")
            continue
        signed = "signed" if pe.get("signed") else "unsigned"
        signers = ", ".join(pe.get("signers", [])[:2])
        a(
            f"- `{pe['path']}` — {pe.get('machine', '?')}, "
            f"{'DLL' if pe.get('is_dll') else 'EXE'}, {signed}"
            + (f" by {signers}" if signers else "")
        )
        dlls = pe.get("import_dlls", [])
        if dlls:
            a(
                f"    - imports {len(dlls)} DLLs: {', '.join(dlls[:12])}"
                + (" ..." if len(dlls) > 12 else "")
            )
        exports = pe.get("exports", [])
        if exports:
            a(f"    - exports {len(exports)} symbols")
    if len(pe_reports) > 50:
        a(f"- _(+{len(pe_reports) - 50} more PEs)_")

    if extraction:
        a("")
        a("## 5. Extraction")
        a("")
        a(
            f"- Extracted files: **{extraction.get('total_files', 0)}** "
            f"(loose: {extraction.get('loose_files', 0)}, "
            f"go-artifacts: {len(extraction.get('go_artifacts', []))})"
        )
        for ar in extraction.get("asar_archives", []):
            if "error" in ar:
                a(f"  - asar `{ar['archive']}` — error: {ar['error']}")
            else:
                a(
                    f"  - asar `{ar['archive']}` — "
                    f"{ar.get('members_written', 0)} members, "
                    f"{ar.get('bytes', 0):,} bytes"
                )

    if documents:
        ok = sum(1 for d in documents if "markdown" in d)
        a("")
        a("## 6. Documents (markitdown)")
        a("")
        a(f"- Converted {ok}/{len(documents)} documents to markdown.")

    if llm:
        a("")
        a("## 7. LLM synthesis (Gemini, keyless)")
        a("")
        if llm.get("synthesis"):
            a(llm["synthesis"])
        if llm.get("answer"):
            a("")
            a("### Q&A")
            a("")
            a(f"**Q:** {llm.get('question', '')}")
            a("")
            a(f"**A:** {llm['answer']}")

    a("")
    return "\n".join(lines)
