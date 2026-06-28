"""RAG backend abstraction — shared types and the backend protocol."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass, field
from typing import Protocol, runtime_checkable


class RagUnavailable(RuntimeError):
    """Raised when a RAG backend's dependencies or services are unavailable.

    Carries an actionable message (e.g. "RAGFlow not reachable, start the
    Docker stack") rather than fabricating an empty/fake result.
    """


@dataclass
class Chunk:
    """A retrieved passage with provenance and a relevance score."""

    text: str
    source: str = ""
    score: float = 0.0


@dataclass
class RagAnswer:
    """A grounded answer plus the chunks it was synthesised from."""

    answer: str
    chunks: list[Chunk] = field(default_factory=list)


@runtime_checkable
class RagBackend(Protocol):
    """Common interface every RAG backend implements."""

    name: str

    def health(self) -> bool:
        """Return True if the backend is ready to serve."""
        ...

    def ingest(self, paths: Sequence[str], *, dataset: str = "default") -> int:
        """Ingest files/dirs into ``dataset``; return the number of items."""
        ...

    def retrieve(
        self, query: str, *, dataset: str = "default", top_k: int = 5
    ) -> list[Chunk]:
        """Return the top-k chunks for ``query``."""
        ...

    def answer(
        self, query: str, *, dataset: str = "default", top_k: int = 5
    ) -> RagAnswer:
        """Retrieve then synthesise a grounded answer with the local LLM."""
        ...


RAG_PROMPT = (
    "You are a precise assistant. Answer the question using ONLY the context "
    "below. If the context is insufficient, say so plainly. Cite sources inline "
    "as [source].\n\n=== CONTEXT ===\n{context}\n\n=== QUESTION ===\n{query}\n"
)


def synthesize(query: str, chunks: Sequence[Chunk]) -> str:
    """Build a grounded answer from chunks using the local OpenAI engine."""
    from ..client import LocalAI

    if not chunks:
        return "No relevant context was retrieved for that query."
    context = "\n\n".join(
        f"[{c.source or f'chunk {i}'}] {c.text}" for i, c in enumerate(chunks)
    )
    prompt = RAG_PROMPT.format(context=context, query=query)
    return LocalAI().chat(prompt)
