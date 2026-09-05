# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.autocomplete` (pure, no network)."""

from __future__ import annotations

import types as _pytypes

import pytest
from aphrody.autocomplete import (
    CodeCompleter,
    CompletionError,
    CompletionRequest,
    build_prompt,
    language_for_path,
    split_at_cursor,
)

from aphrody import autocomplete as ac

# ---------------------------------------------------------------------------
# Fakes mimicking the google-genai response/stream surface.
# ---------------------------------------------------------------------------


def _part(text):
    return _pytypes.SimpleNamespace(text=text, function_call=None)


def _candidate(text, finish="STOP"):
    return _pytypes.SimpleNamespace(
        content=_pytypes.SimpleNamespace(parts=[_part(text)]),
        finish_reason=_pytypes.SimpleNamespace(name=finish),
    )


class _FakeModels:
    def __init__(self, candidate_texts, stream_deltas):
        self._candidate_texts = candidate_texts
        self._stream_deltas = stream_deltas
        self.last_config = None
        self.last_contents = None

    def generate_content(self, *, model, contents, config):
        self.last_config = config
        self.last_contents = contents
        cands = [_candidate(t) for t in self._candidate_texts]
        return _pytypes.SimpleNamespace(candidates=cands, text=None)

    def generate_content_stream(self, *, model, contents, config):
        for d in self._stream_deltas:
            yield _pytypes.SimpleNamespace(text=d)


class _FakeGemini:
    def __init__(self, candidate_texts=("    return n\n",), stream_deltas=()):
        self.model = "gemini-2.5-flash"
        self._models = _FakeModels(list(candidate_texts), list(stream_deltas))

    @property
    def client(self):
        return _pytypes.SimpleNamespace(models=self._models)


def _completer(candidate_texts=("    return n\n",), stream_deltas=()):
    return CodeCompleter(
        gemini=_FakeGemini(candidate_texts, stream_deltas),
        model="gemini-2.5-flash",
    )


# ---------------------------------------------------------------------------
# split_at_cursor
# ---------------------------------------------------------------------------


def test_split_marker():
    pre, suf = split_at_cursor("abc<|cursor|>def", marker="<|cursor|>")
    assert pre == "abc"
    assert suf == "def"


def test_split_line_col():
    text = "line1\nline2\nline3\n"
    pre, suf = split_at_cursor(text, line=2, col=3)
    assert pre == "line1\nli"
    assert suf == "ne2\nline3\n"


def test_split_end_of_text_when_no_position():
    pre, suf = split_at_cursor("hello", marker="<|nope|>")
    assert pre == "hello"
    assert suf == ""


def test_split_line_out_of_range():
    with pytest.raises(CompletionError):
        split_at_cursor("a\nb\n", line=99)


# ---------------------------------------------------------------------------
# prompt building
# ---------------------------------------------------------------------------


def test_build_prompt_continuation():
    system, user = build_prompt(
        CompletionRequest(prefix="def fib(n):", language="python")
    )
    assert "code completion engine" in system.lower()
    assert "continue" in user.lower()
    assert "def fib(n):" in user
    assert "<SUFFIX>" not in user


def test_build_prompt_fim():
    _, user = build_prompt(
        CompletionRequest(prefix="def f():\n", suffix="\n    return 1")
    )
    assert "<PREFIX>" in user
    assert "<SUFFIX>" in user


def test_language_for_path():
    assert language_for_path("foo.py") == "python"
    assert language_for_path("foo.rs") == "rust"
    assert language_for_path("foo.unknownext") is None


# ---------------------------------------------------------------------------
# completion engine (mocked transport)
# ---------------------------------------------------------------------------


def test_complete_returns_candidate():
    engine = _completer(candidate_texts=("    return n * 2\n",))
    res = engine.complete(
        CompletionRequest(prefix="def double(n):", language="python"), n=1
    )
    assert len(res) == 1
    assert res[0].text == "    return n * 2\n"
    assert res[0].model == "gemini-2.5-flash"
    assert res[0].finish_reason == "STOP"


def test_complete_strips_markdown_fence():
    engine = _completer(candidate_texts=("```python\nreturn 1\n```",))
    res = engine.complete(CompletionRequest(prefix="def f():"), n=1)
    assert res[0].text == "return 1"


def test_complete_multiple_candidates():
    engine = _completer(candidate_texts=("a", "b", "c"))
    res = engine.complete(CompletionRequest(prefix="x = "), n=3)
    assert [c.text for c in res] == ["a", "b", "c"]
    assert [c.index for c in res] == [0, 1, 2]


def test_complete_via_module_function_with_prefix():
    engine = _completer(candidate_texts=("pass\n",))
    res = ac.complete(prefix="def noop():", completer=engine)
    assert res[0].text == "pass\n"


def test_complete_requires_exactly_one_source():
    with pytest.raises(CompletionError):
        ac.complete()
    with pytest.raises(CompletionError):
        ac.complete(prefix="x", file="y.py")


def test_stream_yields_deltas():
    engine = _completer(stream_deltas=["re", "turn ", "n"])
    out = "".join(engine.stream(CompletionRequest(prefix="def f(n):")))
    assert out == "return n"


# ---------------------------------------------------------------------------
# file-based completion + the auto loop
# ---------------------------------------------------------------------------


def test_complete_file_with_marker(tmp_path):
    f = tmp_path / "snippet.py"
    f.write_text("def add(a, b):\n    <|cursor|>\n", encoding="utf-8")
    engine = _completer(candidate_texts=("return a + b",))
    res = ac.complete(file=str(f), completer=engine)
    assert res[0].text == "return a + b"


def test_recomplete_file_writes_in_place(tmp_path):
    f = tmp_path / "snippet.py"
    f.write_text("x = <|cursor|>\n", encoding="utf-8")
    engine = _completer(candidate_texts=("42",))
    info = ac.recomplete_file(str(f), completer=engine, write=True)
    assert info["inserted"] == "42"
    assert info["written"] is True
    assert f.read_text(encoding="utf-8") == "x = 42\n"


def test_recomplete_file_missing_marker(tmp_path):
    f = tmp_path / "nomark.py"
    f.write_text("x = 1\n", encoding="utf-8")
    engine = _completer()
    with pytest.raises(CompletionError):
        ac.recomplete_file(str(f), completer=engine)


def test_recomplete_paths_skips_markerless(tmp_path):
    good = tmp_path / "good.py"
    good.write_text("y = <|cursor|>\n", encoding="utf-8")
    bad = tmp_path / "bad.py"
    bad.write_text("z = 1\n", encoding="utf-8")
    engine = _completer(candidate_texts=("7",))
    results = ac.recomplete_paths(
        [str(good), str(bad)], completer=engine, write=False
    )
    assert results[0]["inserted"] == "7"
    assert "skipped" in results[1]
