# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Module 2 — content-type classification with the Magika API.

Runs Google's Magika (Apache-2.0) over each inventoried file —
``Magika().identify_path`` / ``identify_bytes`` — yielding the content-type
label, MIME, group and confidence score, then folds the magika groups together
with the inventory markers into a single per-file *final classification*.

Magika is statically lazy: a single :class:`Magika` instance is reused for the
whole run (one model load), and is injected in tests via ``magika=`` so the
suite never touches the real model.
"""

from __future__ import annotations

import dataclasses
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable

    from aphrody.forensic.inventory import Entry


@dataclasses.dataclass
class Classification:
    """A per-file classification.

    Attributes:
        path: The classified file path.
        label: Magika content-type label (e.g. ``python``, ``pebin``).
        mime_type: Magika MIME type.
        group: Magika group (``code`` / ``document`` / ``image`` / ...).
        score: Magika confidence (0..1).
        is_text: Whether the content is textual.
        markers: Inventory markers carried through for the final category.
        category: The fused final category (see :func:`_final_category`).
    """

    path: str
    label: str
    mime_type: str
    group: str
    score: float
    is_text: bool
    markers: list[str] = dataclasses.field(default_factory=list)
    category: str = "unknown"

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view."""
        return dataclasses.asdict(self)


def get_magika(magika: Any | None = None) -> Any:
    """Return a reusable Magika instance (construct one lazily if needed)."""
    if magika is not None:
        return magika
    from magika import Magika

    return Magika()


def classify_path(
    path: str | Path, *, magika: Any | None = None
) -> Classification:
    """Classify a single file via the Magika API.

    Args:
        path: File to classify.
        magika: A reusable Magika instance (built lazily when omitted).

    Returns:
        A :class:`Classification` (with ``category`` set from magika alone).
    """
    m = get_magika(magika)
    res = m.identify_path(str(path))
    return _from_magika_result(str(path), res)


def _from_magika_result(path: str, res: Any) -> Classification:
    """Build a :class:`Classification` from a Magika result object.

    Supports both the magika 1.x shape (``result.ok`` / ``result.output.label``
    / ``result.score``) and the 0.6.x shape (``result.output.ct_label`` /
    ``result.output.score``, no ``ok``) — markitdown pins magika 0.6.x, so the
    pipeline runs on whichever is installed.
    """
    ok = getattr(res, "ok", None)
    if ok is False:
        return Classification(
            path=path,
            label="error",
            mime_type="",
            group="",
            score=0.0,
            is_text=False,
            category="unreadable",
        )
    out = getattr(res, "output", None)
    if out is None:
        out = getattr(res, "dl", res)

    # Label: 1.x uses ``label``, 0.6.x uses ``ct_label``.
    label = getattr(out, "label", None) or getattr(out, "ct_label", "unknown")

    # Score: 1.x exposes ``result.score`` / ``result.prediction.score``;
    # 0.6.x exposes ``result.output.score``.
    score = getattr(res, "score", None)
    if score is None:
        score = getattr(getattr(res, "prediction", None), "score", None)
    if score is None:
        score = getattr(out, "score", 0.0)

    cls = Classification(
        path=path,
        label=label,
        mime_type=getattr(out, "mime_type", ""),
        group=getattr(out, "group", ""),
        score=float(score or 0.0),
        is_text=bool(getattr(out, "is_text", False)),
    )
    cls.category = _final_category(cls, [])
    return cls


def classify_entries(
    entries: Iterable[Entry], *, magika: Any | None = None
) -> list[Classification]:
    """Classify every non-directory entry and fuse with its markers.

    Args:
        entries: Inventory entries (directories are skipped).
        magika: A reusable Magika instance (built lazily when omitted).

    Returns:
        One :class:`Classification` per file entry.
    """
    m = get_magika(magika)
    out: list[Classification] = []
    for e in entries:
        if e.is_dir:
            continue
        try:
            res = m.identify_path(e.path)
            cls = _from_magika_result(e.path, res)
        except (OSError, ValueError):
            cls = Classification(
                path=e.path,
                label="error",
                mime_type="",
                group="",
                score=0.0,
                is_text=False,
                category="unreadable",
            )
        cls.markers = list(e.markers)
        cls.category = _final_category(cls, e.markers)
        out.append(cls)
    return out


def _final_category(cls: Classification, markers: list[str]) -> str:
    """Fuse magika group + inventory markers into a final coarse category.

    Inventory markers win when they carry forensic intent (a secret file is a
    "secret" regardless of how magika groups its JSON); otherwise the magika
    group drives the category.
    """
    mset = set(markers)
    if "secret" in mset:
        return "secret"
    if "dpapi" in mset or "dpapi_dir" in mset:
        return "dpapi"
    if "go" in mset:
        return "go-binary"
    if "dotnet" in mset:
        return "dotnet-binary"
    if "cpp" in mset:
        return "cpp-binary"
    if "asar" in mset:
        return "electron-asar"
    if "sqlite" in mset:
        return "sqlite-db"
    if "pe" in mset:
        return "pe-binary"
    group = (cls.group or "").lower()
    if group in (
        "code",
        "document",
        "image",
        "audio",
        "video",
        "archive",
        "text",
    ):
        return group
    if cls.label and cls.label not in ("unknown", "empty", "txt"):
        return cls.label
    return "unknown"


def aggregate(classifications: Iterable[Classification]) -> dict[str, Any]:
    """Aggregate classifications by category, group and magika label.

    Args:
        classifications: The per-file classifications.

    Returns:
        A summary with per-category / per-group / per-label counts and a
        breakdown of the binary surface (go / cpp / dotnet / pe).
    """
    by_category: dict[str, int] = {}
    by_group: dict[str, int] = {}
    by_label: dict[str, int] = {}
    total = 0
    for c in classifications:
        total += 1
        by_category[c.category] = by_category.get(c.category, 0) + 1
        if c.group:
            by_group[c.group] = by_group.get(c.group, 0) + 1
        by_label[c.label] = by_label.get(c.label, 0) + 1
    return {
        "total": total,
        "by_category": dict(sorted(by_category.items(), key=lambda kv: -kv[1])),
        "by_group": dict(sorted(by_group.items(), key=lambda kv: -kv[1])),
        "by_label": dict(sorted(by_label.items(), key=lambda kv: -kv[1])),
    }
