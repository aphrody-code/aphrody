# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Forensic pipeline: inventory -> classify -> dll_inspect -> extract -> RAG -> LLM.

A full, unredacted filesystem forensic + classification + RAG + auto-ML LLM
pipeline aimed at the Antigravity IDE install and its associated data
directories. This runs on the owner's own machine against the owner's own data
and the owner's own Google account (keyless) — analysis is fully authorised,
so it reads real values (tokens included), classifies everything, and extracts
everything. Nothing here executes an analysed binary: magika and LIEF are
static, which is the correct reverse-engineering method, not a guardrail.

Modules:
    targets:      well-known Antigravity IDE target directories.
    inventory:    recursive walk, marker detection, real value reads.
    classify:     Magika content-type classification + final categories.
    dll_inspect:  LIEF static PE/DLL inspection (imports/exports/Authenticode).
    reports:      markitdown document -> markdown + consolidated report.
    extract:      asar/extension/config/Go-sidecar source extraction.
    rag:          fastembed embeddings + a local numpy vector store.
    llm:          keyless Gemini synthesis / ask / auto-ML over the corpus.
    pipeline:     the orchestrator wiring 1->7 behind ``aphrody forensic``.
"""

from __future__ import annotations

from aphrody.forensic.pipeline import ForensicPipeline, run_forensic

__all__ = ["ForensicPipeline", "run_forensic"]
