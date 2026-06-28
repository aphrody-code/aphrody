"""Configuration for aphrody-local (env-driven, 12-factor)."""

from __future__ import annotations

import os
from dataclasses import dataclass

# aphrody-serve (the Rust unifying layer) is preferred; Ollama is the fallback.
DEFAULT_SERVE_URL = "http://127.0.0.1:8088/v1"
DEFAULT_OLLAMA_URL = "http://127.0.0.1:11434/v1"


@dataclass
class Settings:
    """Runtime settings, overridable via ``APHRODY_*`` environment variables."""

    base_url: str | None = None  # explicit OpenAI /v1 base; None => auto-discover
    api_key: str = "local"
    model: str | None = None  # None => first available model
    timeout: float = 120.0

    # RAG
    rag_backend: str = "local"  # "local" (fastembed) | "ragflow"
    ragflow_base_url: str = "http://127.0.0.1:9380"
    ragflow_api_key: str = ""
    embed_model: str = "BAAI/bge-small-en-v1.5"
    rerank_model: str = "Xenova/bge-reranker-base"

    @classmethod
    def from_env(cls) -> Settings:
        """Build settings from the environment."""
        return cls(
            base_url=os.getenv("APHRODY_BASE_URL"),
            api_key=os.getenv("APHRODY_API_KEY")
            or os.getenv("OPENAI_API_KEY")
            or "local",
            model=os.getenv("APHRODY_MODEL"),
            timeout=float(os.getenv("APHRODY_TIMEOUT", "120")),
            rag_backend=os.getenv("APHRODY_RAG_BACKEND", "local"),
            ragflow_base_url=os.getenv(
                "RAGFLOW_BASE_URL", "http://127.0.0.1:9380"
            ),
            ragflow_api_key=os.getenv("RAGFLOW_API_KEY", ""),
            embed_model=os.getenv("APHRODY_EMBED_MODEL", "BAAI/bge-small-en-v1.5"),
            rerank_model=os.getenv(
                "APHRODY_RERANK_MODEL", "Xenova/bge-reranker-base"
            ),
        )
