# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Module 6 — local RAG index over the extracted source.

Embeds the extracted corpus with fastembed (Apache-2.0, local ONNX — no API
key, no network at query time once the model is cached) and stores the vectors
in a dependency-free local store: a numpy ``float32`` matrix plus a JSON
sidecar of chunk metadata, persisted under the run directory. ``query(text,
k)`` embeds the question and returns the top-k passages by cosine similarity.

fastembed is imported lazily and injected in tests via ``embedder=`` so the
suite never downloads a model. The vector store is plain numpy.
"""

from __future__ import annotations

import dataclasses
import json
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable, Sequence

#: Default fastembed model (small, fast, CPU-friendly, permissive).
DEFAULT_EMBED_MODEL = "BAAI/bge-small-en-v1.5"

#: Characters per chunk when splitting a source file (overlapping windows).
CHUNK_CHARS = 1200
CHUNK_OVERLAP = 200

#: Skip indexing files bigger than this (already extracted-text only).
MAX_INDEX_BYTES = 4 * 1024 * 1024


@dataclasses.dataclass
class Chunk:
    """One indexed text chunk.

    Attributes:
        doc: Source file path the chunk came from.
        index: Chunk index within that file.
        text: The chunk text.
    """

    doc: str
    index: int
    text: str

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view."""
        return dataclasses.asdict(self)


def chunk_text(
    text: str, *, size: int = CHUNK_CHARS, overlap: int = CHUNK_OVERLAP
) -> list[str]:
    """Split ``text`` into overlapping fixed-size character windows."""
    if not text:
        return []
    if len(text) <= size:
        return [text]
    out: list[str] = []
    step = max(1, size - overlap)
    for start in range(0, len(text), step):
        piece = text[start : start + size]
        if piece.strip():
            out.append(piece)
        if start + size >= len(text):
            break
    return out


def _embedder(embedder: Any | None, model: str) -> Any:
    """Return a fastembed TextEmbedding (or the injected fake)."""
    if embedder is not None:
        return embedder
    from fastembed import TextEmbedding

    return TextEmbedding(model_name=model)


def _embed_texts(embedder: Any, texts: Sequence[str]) -> Any:
    """Embed ``texts`` -> a 2D float32 numpy array (rows = vectors)."""
    import numpy as np

    vectors = list(embedder.embed(list(texts)))
    if not vectors:
        return np.zeros((0, 0), dtype="float32")
    arr = np.asarray(vectors, dtype="float32")
    # L2-normalise so a dot product is cosine similarity.
    norms = np.linalg.norm(arr, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    return arr / norms


class RagIndex:
    """A local numpy-backed RAG index over the extracted corpus.

    The index keeps a ``(n, d)`` float32 matrix of L2-normalised embeddings and
    a parallel list of :class:`Chunk` metadata. Persisted as ``vectors.npy`` +
    ``chunks.json`` under the index directory.
    """

    def __init__(
        self,
        *,
        model: str = DEFAULT_EMBED_MODEL,
        embedder: Any | None = None,
    ) -> None:
        self._model = model
        self._embedder_obj = embedder
        self._vectors: Any = None  # numpy array, lazily created
        self._chunks: list[Chunk] = []

    @property
    def embedder(self) -> Any:
        """The lazily-built fastembed embedder (or injected fake)."""
        if self._embedder_obj is None:
            self._embedder_obj = _embedder(None, self._model)
        return self._embedder_obj

    @property
    def size(self) -> int:
        """Number of indexed chunks."""
        return len(self._chunks)

    def add_files(
        self, paths: Iterable[str | Path], *, batch: int = 256
    ) -> int:
        """Chunk + embed every text file in ``paths``; add to the index.

        Args:
            paths: File paths to index (non-text / oversized files are skipped).
            batch: Embedding batch size.

        Returns:
            The number of chunks added.
        """
        import numpy as np

        texts: list[str] = []
        metas: list[Chunk] = []
        for p in paths:
            path = Path(p)
            try:
                if path.stat().st_size > MAX_INDEX_BYTES:
                    continue
                content = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for i, piece in enumerate(chunk_text(content)):
                texts.append(piece)
                metas.append(Chunk(doc=str(path), index=i, text=piece))

        if not texts:
            return 0

        all_vecs = []
        for start in range(0, len(texts), batch):
            sub = texts[start : start + batch]
            all_vecs.append(_embed_texts(self.embedder, sub))
        new = np.vstack(all_vecs)

        if self._vectors is None or self._vectors.size == 0:
            self._vectors = new
        else:
            self._vectors = np.vstack([self._vectors, new])
        self._chunks.extend(metas)
        return len(metas)

    def query(self, text: str, *, k: int = 5) -> list[dict[str, Any]]:
        """Return the top-``k`` passages most similar to ``text``.

        Args:
            text: The query text.
            k: Number of passages to return.

        Returns:
            ``[{doc, index, score, text}]`` ordered by descending similarity.
        """
        import numpy as np

        if self._vectors is None or self.size == 0:
            return []
        q = _embed_texts(self.embedder, [text])  # (1, d), normalised
        if q.shape[1] != self._vectors.shape[1]:
            return []
        scores = self._vectors @ q[0]  # cosine (both normalised)
        k = min(k, self.size)
        top = np.argsort(-scores)[:k]
        out: list[dict[str, Any]] = []
        for idx in top:
            c = self._chunks[int(idx)]
            out.append(
                {
                    "doc": c.doc,
                    "index": c.index,
                    "score": float(scores[int(idx)]),
                    "text": c.text,
                }
            )
        return out

    def save(self, index_dir: str | Path) -> dict[str, Any]:
        """Persist the index (``vectors.npy`` + ``chunks.json``)."""
        import numpy as np

        d = Path(index_dir)
        d.mkdir(parents=True, exist_ok=True)
        if self._vectors is not None and self._vectors.size:
            np.save(d / "vectors.npy", self._vectors)
        (d / "chunks.json").write_text(
            json.dumps(
                {
                    "model": self._model,
                    "chunks": [c.to_dict() for c in self._chunks],
                },
                ensure_ascii=False,
            ),
            encoding="utf-8",
        )
        return {"index_dir": str(d), "chunks": self.size}

    @classmethod
    def load(
        cls,
        index_dir: str | Path,
        *,
        embedder: Any | None = None,
    ) -> RagIndex:
        """Load a persisted index from ``index_dir``."""
        import numpy as np

        d = Path(index_dir)
        meta = json.loads((d / "chunks.json").read_text(encoding="utf-8"))
        idx = cls(
            model=meta.get("model", DEFAULT_EMBED_MODEL), embedder=embedder
        )
        idx._chunks = [
            Chunk(doc=c["doc"], index=c["index"], text=c["text"])
            for c in meta.get("chunks", [])
        ]
        vec_path = d / "vectors.npy"
        if vec_path.exists():
            idx._vectors = np.load(vec_path)
        return idx


def build_index(
    files: Iterable[str | Path],
    *,
    model: str = DEFAULT_EMBED_MODEL,
    embedder: Any | None = None,
) -> RagIndex:
    """Build a :class:`RagIndex` from ``files`` (convenience wrapper)."""
    index = RagIndex(model=model, embedder=embedder)
    index.add_files(files)
    return index
