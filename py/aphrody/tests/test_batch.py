# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.batch` using a fake generator (no network)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from aphrody import batch


def test_resolve_prompt_literal() -> None:
    item = batch.BatchItem(id="a", prompt="a red banana")
    assert item.resolve_prompt() == "a red banana"


def test_resolve_prompt_template_and_enhance() -> None:
    item = batch.BatchItem(
        id="logo",
        template="logo",
        vars={"brand_name": "Aphrody"},
        enhance="product",
    )
    out = item.resolve_prompt()
    assert '"Aphrody"' in out
    # enhancer appended a product-preset modifier
    assert "product photography" in out


def test_resolve_prompt_requires_exactly_one_source() -> None:
    with pytest.raises(ValueError, match="exactly one"):
        batch.BatchItem(id="a").resolve_prompt()
    with pytest.raises(ValueError, match="exactly one"):
        batch.BatchItem(id="a", prompt="x", template="logo").resolve_prompt()


def test_coerce_item_rejects_unknown_keys() -> None:
    with pytest.raises(ValueError, match="unknown item keys"):
        batch._coerce_item({"id": "a", "prompt": "x", "bogus": 1})


def test_coerce_item_requires_id() -> None:
    with pytest.raises(ValueError, match="requires an 'id'"):
        batch._coerce_item({"prompt": "x"})


def test_load_spec(tmp_path) -> None:
    spec = {
        "defaults": {"image_size": "2K"},
        "items": [
            {"id": "a", "prompt": "x"},
            {"id": "b", "template": "logo", "vars": {"brand_name": "Z"}},
        ],
    }
    p = tmp_path / "spec.json"
    p.write_text(json.dumps(spec), encoding="utf-8")
    defaults, items = batch.load_spec(p)
    assert defaults == {"image_size": "2K"}
    assert [i.id for i in items] == ["a", "b"]


def test_apply_defaults_fills_unset() -> None:
    item = batch.BatchItem(id="a", prompt="x")
    batch._apply_defaults(item, {"image_size": "4K", "optimize": ["png"]})
    assert item.image_size == "4K"
    assert item.optimize == ("png",)


class _FakeNanoBanana:
    """Stand-in that writes a tiny PNG instead of calling the network."""

    def __init__(self, **_kw: object) -> None:
        self.last_model = "fake-image-model"

    def generate_image(self, prompt: str, *, out, n, **_kw):
        dest = Path(out)
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(b"\x89PNG\r\n\x1a\n")
        return [dest]


def test_generate_batch_writes_files_and_manifest(
    tmp_path, monkeypatch
) -> None:
    monkeypatch.setattr(batch, "NanoBanana", _FakeNanoBanana)
    items = [
        batch.BatchItem(id="alpha", prompt="x"),
        batch.BatchItem(id="beta", template="logo", vars={"brand_name": "Z"}),
    ]
    results = batch.generate_batch(items, out_dir=tmp_path, max_workers=2)

    assert len(results) == 2
    assert all(r.ok for r in results)
    assert {r.id for r in results} == {"alpha", "beta"}
    assert (tmp_path / "alpha.png").exists()
    assert (tmp_path / "manifest.json").exists()
    manifest = json.loads((tmp_path / "manifest.json").read_text())
    assert manifest["ok"] == 2
    assert manifest["failed"] == 0


def test_generate_batch_isolates_item_failure(tmp_path, monkeypatch) -> None:
    monkeypatch.setattr(batch, "NanoBanana", _FakeNanoBanana)
    items = [
        batch.BatchItem(id="good", prompt="x"),
        batch.BatchItem(id="bad"),  # no prompt/template -> resolve error
    ]
    results = batch.generate_batch(
        items, out_dir=tmp_path, max_workers=2, write_manifest=False
    )
    by_id = {r.id: r for r in results}
    assert by_id["good"].ok
    assert not by_id["bad"].ok
    assert "exactly one" in by_id["bad"].error
