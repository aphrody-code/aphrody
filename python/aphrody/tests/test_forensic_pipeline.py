# SPDX-License-Identifier: Apache-2.0
"""End-to-end pipeline tests with every backend mocked (no network/models)."""

from __future__ import annotations

import json
import types as _t
from typing import ClassVar

import numpy as np
from aphrody.forensic import inventory
from aphrody.forensic.pipeline import ForensicPipeline


class _FakeOut:
    def __init__(self, label, mime, group, is_text):
        self.label, self.mime_type, self.group, self.is_text = (
            label,
            mime,
            group,
            is_text,
        )


class _FakeResult:
    def __init__(self, label, mime, group, is_text, score):
        self.ok = True
        self.output = _FakeOut(label, mime, group, is_text)
        self.score = score
        self.prediction = _t.SimpleNamespace(score=score)


class _FakeMagika:
    def identify_path(self, path):
        p = str(path)
        if p.endswith(".py"):
            return _FakeResult("python", "text/x-python", "code", True, 0.99)
        if p.endswith(".json"):
            return _FakeResult("json", "application/json", "code", True, 0.95)
        if p.endswith(".exe"):
            return _FakeResult(
                "pebin", "application/x-dosexec", "executable", False, 0.9
            )
        return _FakeResult("txt", "text/plain", "text", True, 0.5)


def _fake_lief_module():
    binary = _t.SimpleNamespace(
        header=_t.SimpleNamespace(
            machine="AMD64", characteristics_list=["EXECUTABLE_IMAGE"]
        ),
        optional_header=_t.SimpleNamespace(subsystem="CUI"),
        imports=[
            _t.SimpleNamespace(
                name="KERNEL32.dll",
                entries=[
                    _t.SimpleNamespace(name="CreateFileW", is_ordinal=False)
                ],
            )
        ],
        get_export=lambda: _t.SimpleNamespace(entries=[]),
        sections=[
            _t.SimpleNamespace(
                name=".text", virtual_size=10, size=10, entropy=6.0
            )
        ],
        has_signatures=lambda: True,
        signatures=[
            _t.SimpleNamespace(
                signers=[
                    _t.SimpleNamespace(
                        cert=_t.SimpleNamespace(subject="O=Google Inc")
                    )
                ]
            )
        ],
    )
    return _t.SimpleNamespace(parse=lambda path: binary)


class _FakeEmbedder:
    VOCAB: ClassVar = ["auth", "agent", "render"]

    def embed(self, texts):
        for t in texts:
            low = t.lower()
            v = [float(low.count(w)) for w in self.VOCAB]
            if not any(v):
                v[0] = 0.001
            yield np.asarray(v, dtype="float32")


class _FakeMarkItDown:
    def convert(self, path):
        return _t.SimpleNamespace(text_content="# Doc\n\nconverted body")


class _FakeLLM:
    def synthesize(self, **kw):
        return "## Synthesis\nElectron + Go LS."

    def auto_ml(self, **kw):
        return {"architecture": "shell+ls", "components": [], "raw": "{}"}

    def ask(self, question, **kw):
        return {
            "question": question,
            "answer": "Because OAuth.",
            "passages": [],
        }


def _make_target(tmp_path):
    root = tmp_path / "Antigravity IDE"
    root.mkdir()
    (root / "main.py").write_text("print('agent auth')", encoding="utf-8")
    (root / "product.json").write_text(
        json.dumps({"version": "2.0.2", "nameLong": "Antigravity IDE"}),
        encoding="utf-8",
    )
    (root / "oauth_creds.json").write_text(
        json.dumps({"access_token": "ya29.SECRET", "scope": "cloud-platform"}),
        encoding="utf-8",
    )
    (root / "ls.exe").write_bytes(
        b"MZ" + b"\x00" * 64 + inventory.GO_BUILDINFO_MAGIC + b"go1.23"
    )
    return root


def test_pipeline_dry_run(tmp_path):
    root = _make_target(tmp_path)
    out = tmp_path / "out"
    pipe = ForensicPipeline(
        target=str(root),
        dry_run=True,
        out_dir=str(out),
        magika=_FakeMagika(),
        lief_mod=_fake_lief_module(),
        md=_FakeMarkItDown(),
    )
    report = pipe.run()

    assert report["exists"] is True
    assert report["dry_run"] is True
    assert "llm" not in report  # dry run skips the model
    # Inventory found the secret with its real value.
    sec_paths = [
        s["path"] for s in report["inventory"]["summary"]["secret_files"]
    ]
    assert any("oauth_creds.json" in p for p in sec_paths)
    # Go binary classified.
    assert report["classification"]["by_category"].get("go-binary", 0) == 1
    # PE inspected.
    assert report["pe_reports"]
    assert report["pe_reports"][0]["signed"] is True
    # Reports written.
    assert (out / "report.json").exists()
    assert (out / "report.md").exists()
    md = (out / "report.md").read_text(encoding="utf-8")
    assert "Forensic report" in md
    # report.json is fully serialisable & contains the real token value.
    data = json.loads((out / "report.json").read_text(encoding="utf-8"))
    creds = next(
        e
        for e in data["inventory"]["entries"]
        if e["path"].endswith("oauth_creds.json")
    )
    assert creds["details"]["secret"]["access_token"] == "ya29.SECRET"


def test_pipeline_deep_with_llm(tmp_path):
    root = _make_target(tmp_path)
    out = tmp_path / "out"
    pipe = ForensicPipeline(
        target=str(root),
        deep=True,
        dry_run=False,
        ask="Why OAuth?",
        out_dir=str(out),
        magika=_FakeMagika(),
        lief_mod=_fake_lief_module(),
        md=_FakeMarkItDown(),
        embedder=_FakeEmbedder(),
        llm=_FakeLLM(),
    )
    report = pipe.run()

    assert "extraction" in report
    assert "rag" in report
    assert report["rag"]["chunks"] >= 1
    assert report["llm"]["synthesis"].startswith("## Synthesis")
    assert report["llm"]["auto_ml"]["architecture"] == "shell+ls"
    assert report["llm"]["answer"] == "Because OAuth."
    assert (out / "rag-index" / "chunks.json").exists()


def test_pipeline_missing_target(tmp_path):
    pipe = ForensicPipeline(
        target=str(tmp_path / "nope"),
        dry_run=True,
        out_dir=str(tmp_path / "out"),
        magika=_FakeMagika(),
    )
    report = pipe.run()
    assert report["exists"] is False
    assert "error" in report
