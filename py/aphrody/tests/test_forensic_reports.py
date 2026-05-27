# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.reports` (mocked markitdown)."""

from __future__ import annotations

import types as _t

from aphrody.forensic import reports


class _FakeMarkItDown:
    def __init__(self, text="# Converted\n\nbody"):
        self._text = text

    def convert(self, path):
        return _t.SimpleNamespace(text_content=self._text)


def test_convert_document():
    res = reports.convert_document("doc.pdf", md=_FakeMarkItDown())
    assert "Converted" in res["markdown"]


def test_convert_document_error():
    class _Boom:
        def convert(self, path):
            raise RuntimeError("bad pdf")

    res = reports.convert_document("doc.pdf", md=_Boom())
    assert "bad pdf" in res["error"]


def test_convert_documents_writes_md(tmp_path):
    pdf = tmp_path / "report.pdf"
    pdf.write_bytes(b"%PDF-1.7 fake")
    entries = [_t.SimpleNamespace(path=str(pdf), ext="pdf", is_dir=False)]
    out = reports.convert_documents(
        entries, tmp_path / "out", md=_FakeMarkItDown()
    )
    assert len(out) == 1
    assert out[0]["md_path"].endswith("report.pdf.md")


def test_build_markdown_report_full():
    inv = {
        "summary": {
            "files": 10,
            "dirs": 2,
            "total_bytes": 1234,
            "markers": {"go": 1, "secret": 1},
            "secret_files": [
                {
                    "path": "oauth_creds.json",
                    "token_type": "json",
                    "scope": "cloud-platform",
                }
            ],
        }
    }
    cls = {"by_category": {"code": 5, "go-binary": 1, "secret": 1}}
    pe = [
        {
            "path": "ls.exe",
            "machine": "AMD64",
            "is_dll": False,
            "signed": True,
            "signers": ["O=Google Inc"],
            "import_dlls": ["KERNEL32.dll"],
            "exports": [],
        }
    ]
    md = reports.build_markdown_report(
        target="install",
        inventory=inv,
        classification=cls,
        pe_reports=pe,
        extraction={
            "total_files": 7,
            "loose_files": 5,
            "go_artifacts": [],
            "asar_archives": [],
        },
        documents=[{"markdown": "x"}],
        llm={"synthesis": "Big synthesis", "question": "q?", "answer": "a."},
    )
    assert "# Forensic report" in md
    assert "oauth_creds.json" in md
    assert "go-binary" in md
    assert "Google Inc" in md
    assert "Big synthesis" in md
    assert "q?" in md
