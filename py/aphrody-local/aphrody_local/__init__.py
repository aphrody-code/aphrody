"""aphrody-local — local open-weight AI orchestrator.

Python complement to the Rust ``aphrody-serve`` OpenAI server: chat/completions
against local engines, plus RAG (RAGFlow or a torch-free local fallback),
engine discovery, and model selection.
"""

from __future__ import annotations

from .client import LocalAI
from .config import Settings

__version__ = "0.1.0"
__all__ = ["LocalAI", "Settings", "__version__"]
