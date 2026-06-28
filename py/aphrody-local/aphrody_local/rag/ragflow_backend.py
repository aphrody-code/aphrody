"""RAGFlow backend — real ``ragflow-sdk`` client, gated on availability.

RAGFlow (https://github.com/infiniflow/ragflow) is a deep-document-understanding
RAG engine deployed via Docker Compose (Elasticsearch/Infinity + MySQL + Redis +
MinIO + ragflow-server; needs ~16 GB RAM, ~50 GB disk). Configure its model
providers to "OpenAI-API-Compatible" pointed at the local engine
(``http://host.docker.internal:8088/v1`` or Ollama) so no cloud keys are used.

This backend never fabricates results: if the SDK is missing, the API key is
unset, or the server is unreachable, it raises :class:`RagUnavailable` with an
actionable message. SDK method names track the installed ``ragflow-sdk``; verify
against the version pinned in ``pyproject.toml`` when the stack is live.
"""

from __future__ import annotations

import os
from collections.abc import Sequence

from ..config import Settings
from .base import Chunk, RagAnswer, RagUnavailable, synthesize


class RagflowBackend:
    """Thin wrapper over the official ``ragflow-sdk``."""

    name = "ragflow"

    def __init__(self, settings: Settings | None = None) -> None:
        self.settings = settings or Settings.from_env()
        self._client = None

    def _rag(self):
        """Return a connected RAGFlow client or raise :class:`RagUnavailable`."""
        if self._client is not None:
            return self._client
        try:
            from ragflow_sdk import RAGFlow
        except ImportError as exc:
            raise RagUnavailable(
                "ragflow-sdk not installed — `pip install ragflow-sdk`"
            ) from exc
        if not self.settings.ragflow_api_key:
            raise RagUnavailable(
                "RAGFLOW_API_KEY not set. Start the RAGFlow Docker stack and "
                "create an API key in its UI (default http://localhost:9380), "
                "then export RAGFLOW_API_KEY."
            )
        self._client = RAGFlow(
            api_key=self.settings.ragflow_api_key,
            base_url=self.settings.ragflow_base_url,
        )
        return self._client

    def _dataset(self, rag, name: str):
        existing = rag.list_datasets(name=name)
        if existing:
            return existing[0]
        return rag.create_dataset(name=name)

    def health(self) -> bool:
        try:
            self._rag().list_datasets()
        except Exception:  # noqa: BLE001 — health must never raise
            return False
        return True

    def ingest(self, paths: Sequence[str], *, dataset: str = "default") -> int:
        rag = self._rag()
        ds = self._dataset(rag, dataset)
        docs = []
        for raw in paths:
            with open(os.path.expanduser(raw), "rb") as fh:
                docs.append(
                    {"display_name": os.path.basename(raw), "blob": fh.read()}
                )
        if not docs:
            return 0
        ds.upload_documents(docs)
        # Kick off async parsing for everything not yet chunked.
        doc_ids = [d.id for d in ds.list_documents()]
        ds.async_parse_documents(doc_ids)
        return len(docs)

    def retrieve(
        self, query: str, *, dataset: str = "default", top_k: int = 5
    ) -> list[Chunk]:
        rag = self._rag()
        ds = self._dataset(rag, dataset)
        results = rag.retrieve(
            question=query, dataset_ids=[ds.id], page_size=top_k
        )
        return [
            Chunk(
                text=getattr(c, "content", ""),
                source=getattr(c, "document_keyword", "")
                or getattr(c, "document_id", ""),
                score=float(getattr(c, "similarity", 0.0) or 0.0),
            )
            for c in results
        ]

    def answer(
        self, query: str, *, dataset: str = "default", top_k: int = 5
    ) -> RagAnswer:
        chunks = self.retrieve(query, dataset=dataset, top_k=top_k)
        return RagAnswer(answer=synthesize(query, chunks), chunks=chunks)
