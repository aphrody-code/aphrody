"""RAG backends for aphrody-local."""

from __future__ import annotations

from .base import Chunk, RagAnswer, RagBackend, RagUnavailable
from .config_factory import get_backend

__all__ = [
    "Chunk",
    "RagAnswer",
    "RagBackend",
    "RagUnavailable",
    "get_backend",
]
