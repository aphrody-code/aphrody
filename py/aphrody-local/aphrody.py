#!/usr/bin/env python3
"""aphrody.py — runnable entry for the *aphrody-local* orchestrator.

A "real local aphrody": a thin Python brain over this machine's open-weight
stack. It talks to the Rust ``aphrody-serve`` OpenAI server (or any local
OpenAI-compatible engine — Ollama / vLLM / llama.cpp) and adds the ML/RAG/data
surface that Python owns (CLAUDE.md §2), with RAGFlow as the heavyweight RAG
engine and a torch-free local fallback.

Usage
-----
    python aphrody.py chat "say hi in 3 words"
    python aphrody.py models
    python aphrody.py engines
    python aphrody.py doctor
    python aphrody.py rag ingest ./notes
    python aphrody.py rag query "what did I write about X?"
"""

from __future__ import annotations

import sys
from pathlib import Path

# Allow running as a loose script (no install): make the package importable.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from aphrody_local.cli import app  # noqa: E402

if __name__ == "__main__":
    app()
