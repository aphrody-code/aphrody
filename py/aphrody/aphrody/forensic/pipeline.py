# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""The forensic pipeline orchestrator (modules 1 -> 7).

Wires inventory -> classify -> dll_inspect -> extract -> reports -> rag -> llm
behind a single entry point and writes the JSON + markdown report under
``var/data/forensic-<target>/``. ``--dry-run`` skips the LLM call (offline
smoke); ``--deep`` enables the heavy passes (full extraction + RAG indexing,
and the LLM auto-ML tagging when not dry).
"""

from __future__ import annotations

import dataclasses
import json
import re
from pathlib import Path
from typing import Any

from aphrody import _paths
from aphrody.forensic import classify as classify_mod
from aphrody.forensic import dll_inspect, extract, inventory, reports, targets
from aphrody.forensic import rag as rag_mod


def _slug(target: str) -> str:
    """Slugify a target name/path for the output directory."""
    base = targets.resolve_target(target).name or "target"
    return re.sub(r"[^A-Za-z0-9_.-]+", "-", base).strip("-").lower() or "target"


def _output_dir(target: str) -> Path:
    """Resolve ``<repo>/var/data/forensic-<slug>`` (repo-aware, gitignored)."""
    secrets = _paths.secrets_dir()  # <repo>/var/secrets when in-repo
    var = secrets.parent if secrets.name == "secrets" else secrets
    return var / "data" / f"forensic-{_slug(target)}"


@dataclasses.dataclass
class ForensicPipeline:
    """Configurable forensic pipeline runner.

    Attributes:
        target: Target name or path.
        deep: Enable heavy passes (extraction + RAG + auto-ML tagging).
        dry_run: Skip every LLM call (offline smoke).
        out_dir: Override the output directory.
        max_files: Inventory file cap.
        magika: Injected Magika instance (tests).
        lief_mod: Injected LIEF module (tests).
        embedder: Injected fastembed embedder (tests).
        md: Injected MarkItDown instance (tests).
        llm: Injected ForensicLLM (tests).
    """

    target: str
    deep: bool = False
    dry_run: bool = False
    out_dir: str | None = None
    max_files: int = 200_000
    ask: str | None = None
    magika: Any | None = None
    lief_mod: Any | None = None
    embedder: Any | None = None
    md: Any | None = None
    llm: Any | None = None

    def run(self) -> dict[str, Any]:
        """Run the pipeline end-to-end and write the reports.

        Returns:
            The consolidated report dict (also written as ``report.json``).
        """
        target_path = targets.resolve_target(self.target)
        out = Path(self.out_dir) if self.out_dir else _output_dir(self.target)
        out.mkdir(parents=True, exist_ok=True)

        report: dict[str, Any] = {
            "target": self.target,
            "resolved_path": str(target_path),
            "exists": target_path.exists(),
            "deep": self.deep,
            "dry_run": self.dry_run,
            "out_dir": str(out),
        }

        if not target_path.exists():
            report["error"] = "target path does not exist"
            self._write_json(out, report)
            return report

        # 1. Inventory.
        inv = inventory.walk_inventory(target_path, max_files=self.max_files)
        report["inventory"] = inv.to_dict()

        # 2. Classify (Magika).
        classifications = classify_mod.classify_entries(
            inv.entries, magika=self.magika
        )
        report["classification"] = classify_mod.aggregate(classifications)
        report["classifications"] = [c.to_dict() for c in classifications]

        # 3. PE / DLL inspection (LIEF).
        pe_reports = dll_inspect.inspect_entries(
            inv.entries, lief_mod=self.lief_mod
        )
        report["pe_reports"] = [p.to_dict() for p in pe_reports]

        # 4. Documents (markitdown).
        documents = reports.convert_documents(inv.entries, out, md=self.md)
        report["documents"] = [
            {k: v for k, v in d.items() if k != "markdown"} for d in documents
        ]

        # 5. Extraction + 6. RAG (heavy: deep only).
        rag_index = None
        extraction = None
        if self.deep:
            extraction = extract.extract_all(inv.entries, out)
            report["extraction"] = extraction.to_dict()
            corpus = list(extraction.files)
            corpus += [d["md_path"] for d in documents if d.get("md_path")]
            rag_index = rag_mod.RagIndex(embedder=self.embedder)
            try:
                added = rag_index.add_files(corpus)
                report["rag"] = {"chunks": added, "files": len(corpus)}
                rag_index.save(out / "rag-index")
            except (ImportError, RuntimeError) as exc:
                report["rag"] = {"error": str(exc), "files": len(corpus)}
                rag_index = None

        # 7. LLM synthesis / ask / auto-ML (skipped on dry-run).
        if not self.dry_run:
            llm = self.llm
            if llm is None:
                from aphrody.forensic.llm import ForensicLLM

                llm = ForensicLLM()
            llm_out: dict[str, Any] = {}
            llm_out["synthesis"] = llm.synthesize(
                inventory=report["inventory"],
                classification=report["classification"],
                pe_reports=report["pe_reports"],
                rag=rag_index,
            )
            if self.deep:
                llm_out["auto_ml"] = llm.auto_ml(
                    inventory=report["inventory"],
                    classification=report["classification"],
                    pe_reports=report["pe_reports"],
                    rag=rag_index,
                )
            if self.ask:
                qa = llm.ask(self.ask, rag=rag_index)
                llm_out.update(qa)
            report["llm"] = llm_out

        # Consolidated markdown report.
        md_text = reports.build_markdown_report(
            target=self.target,
            inventory=report["inventory"],
            classification=report["classification"],
            pe_reports=report["pe_reports"],
            extraction=report.get("extraction"),
            documents=documents,
            llm=report.get("llm"),
        )
        (out / "report.md").write_text(md_text, encoding="utf-8")
        report["report_md"] = str(out / "report.md")

        self._write_json(out, report)
        return report

    @staticmethod
    def _write_json(out: Path, report: dict[str, Any]) -> None:
        """Write the JSON report (UTF-8, indented)."""
        (out / "report.json").write_text(
            json.dumps(report, indent=2, ensure_ascii=False),
            encoding="utf-8",
        )


def run_forensic(
    target: str,
    *,
    deep: bool = False,
    dry_run: bool = False,
    ask: str | None = None,
    out_dir: str | None = None,
    max_files: int = 200_000,
) -> dict[str, Any]:
    """Run the forensic pipeline on ``target`` (convenience wrapper).

    Args:
        target: Target name (``install`` / ``appdata`` / ...) or a path.
        deep: Enable extraction + RAG + auto-ML tagging.
        dry_run: Skip the LLM call (offline smoke).
        ask: An optional question answered via RAG -> Gemini.
        out_dir: Override the output directory.
        max_files: Inventory file cap.

    Returns:
        The consolidated report dict.
    """
    return ForensicPipeline(
        target=target,
        deep=deep,
        dry_run=dry_run,
        ask=ask,
        out_dir=out_dir,
        max_files=max_files,
    ).run()
