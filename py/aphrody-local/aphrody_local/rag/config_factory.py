"""Factory that selects a RAG backend from settings."""

from __future__ import annotations

from ..config import Settings
from .base import RagBackend


def get_backend(settings: Settings | None = None) -> RagBackend:
    """Return the configured RAG backend instance.

    ``APHRODY_RAG_BACKEND=ragflow`` selects the heavyweight RAGFlow engine;
    anything else (default) selects the torch-free local fallback.
    """
    settings = settings or Settings.from_env()
    if settings.rag_backend.lower() == "ragflow":
        from .ragflow_backend import RagflowBackend

        return RagflowBackend(settings)
    from .local_backend import LocalRagBackend

    return LocalRagBackend(settings)
