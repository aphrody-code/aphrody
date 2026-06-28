"""Torch-free local RAG backend.

Embeddings and reranking run on ONNX via ``fastembed`` (no PyTorch), so this
works in a light venv without touching ``~/ml-env``. Vectors are persisted as
JSON under ``~/.aphrody-local/rag/<dataset>.json``. Retrieval is exact cosine
similarity (fine for personal-scale corpora); an optional cross-encoder rerank
sharpens the top results.
"""

from __future__ import annotations

import json
import math
import os
from collections.abc import Sequence
from pathlib import Path

from ..config import Settings
from .base import Chunk, RagAnswer, RagUnavailable, synthesize

STORE_DIR = Path(os.path.expanduser("~/.aphrody-local/rag"))
_TEXT_SUFFIXES = {".txt", ".md", ".markdown", ".rst", ".py", ".json", ".csv"}


def _chunk_text(text: str, size: int = 800, overlap: int = 120) -> list[str]:
    """Split text into overlapping character windows on whitespace bounds."""
    text = " ".join(text.split())
    if len(text) <= size:
        return [text] if text else []
    chunks: list[str] = []
    start = 0
    while start < len(text):
        end = min(start + size, len(text))
        # back off to the last space to avoid cutting words
        if end < len(text):
            space = text.rfind(" ", start, end)
            if space > start:
                end = space
        chunks.append(text[start:end].strip())
        if end >= len(text):
            break
        start = max(end - overlap, start + 1)
    return [c for c in chunks if c]


def _iter_files(paths: Sequence[str]):
    for raw in paths:
        p = Path(raw).expanduser()
        if p.is_dir():
            yield from (f for f in p.rglob("*") if f.is_file())
        elif p.is_file():
            yield p


def _read_file(path: Path) -> str:
    if path.suffix.lower() == ".pdf":
        try:
            from pypdf import PdfReader
        except ImportError:
            return ""
        try:
            return "\n".join(
                page.extract_text() or "" for page in PdfReader(str(path)).pages
            )
        except Exception:  # noqa: BLE001 — a single bad PDF must not abort ingest
            return ""
    if path.suffix.lower() in _TEXT_SUFFIXES:
        try:
            return path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            return ""
    return ""


def _cosine(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b, strict=False))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(y * y for y in b))
    return dot / (na * nb) if na and nb else 0.0


class LocalRagBackend:
    """fastembed-backed local RAG (embed → cosine retrieve → rerank → answer)."""

    name = "local"

    def __init__(self, settings: Settings | None = None) -> None:
        self.settings = settings or Settings.from_env()
        self._embedder = None
        self._reranker = None
        STORE_DIR.mkdir(parents=True, exist_ok=True)

    # -- model loaders (lazy; fastembed downloads ONNX weights on first use) --
    def _embed(self, texts: list[str]) -> list[list[float]]:
        if self._embedder is None:
            try:
                from fastembed import TextEmbedding
            except ImportError as exc:
                raise RagUnavailable(
                    "fastembed not installed — `pip install fastembed`"
                ) from exc
            self._embedder = TextEmbedding(model_name=self.settings.embed_model)
        return [vec.tolist() for vec in self._embedder.embed(texts)]

    def _rerank(self, query: str, docs: list[str]) -> list[float] | None:
        try:
            from fastembed.rerank.cross_encoder import TextCrossEncoder
        except ImportError:
            return None
        if self._reranker is None:
            self._reranker = TextCrossEncoder(model_name=self.settings.rerank_model)
        return list(self._reranker.rerank(query, docs))

    # -- store helpers --
    def _store_path(self, dataset: str) -> Path:
        safe = "".join(c if c.isalnum() or c in "-_" else "_" for c in dataset)
        return STORE_DIR / f"{safe}.json"

    def _load(self, dataset: str) -> dict:
        path = self._store_path(dataset)
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
        return {"model": self.settings.embed_model, "items": []}

    def _save(self, dataset: str, store: dict) -> None:
        self._store_path(dataset).write_text(
            json.dumps(store), encoding="utf-8"
        )

    # -- RagBackend protocol --
    def health(self) -> bool:
        try:
            import fastembed  # noqa: F401
        except ImportError:
            return False
        return True

    def ingest(self, paths: Sequence[str], *, dataset: str = "default") -> int:
        store = self._load(dataset)
        new_items: list[dict] = []
        texts: list[str] = []
        sources: list[str] = []
        for f in _iter_files(paths):
            content = _read_file(f)
            for chunk in _chunk_text(content):
                texts.append(chunk)
                sources.append(f.name)
        if not texts:
            return 0
        vectors = self._embed(texts)
        for text, source, vec in zip(texts, sources, vectors, strict=True):
            new_items.append({"text": text, "source": source, "vec": vec})
        store["items"].extend(new_items)
        self._save(dataset, store)
        return len(new_items)

    def retrieve(
        self, query: str, *, dataset: str = "default", top_k: int = 5
    ) -> list[Chunk]:
        store = self._load(dataset)
        items = store.get("items", [])
        if not items:
            return []
        qvec = self._embed([query])[0]
        scored = sorted(
            (
                Chunk(it["text"], it.get("source", ""), _cosine(qvec, it["vec"]))
                for it in items
            ),
            key=lambda c: c.score,
            reverse=True,
        )
        # over-fetch, then optionally rerank with a cross-encoder
        pool = scored[: max(top_k * 4, top_k)]
        rerank_scores = self._rerank(query, [c.text for c in pool])
        if rerank_scores is not None:
            for chunk, rscore in zip(pool, rerank_scores, strict=False):
                chunk.score = float(rscore)
            pool.sort(key=lambda c: c.score, reverse=True)
        return pool[:top_k]

    def answer(
        self, query: str, *, dataset: str = "default", top_k: int = 5
    ) -> RagAnswer:
        chunks = self.retrieve(query, dataset=dataset, top_k=top_k)
        return RagAnswer(answer=synthesize(query, chunks), chunks=chunks)
