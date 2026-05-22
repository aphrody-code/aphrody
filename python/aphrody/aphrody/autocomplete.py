# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Keyless automatic code completion ("auto-recomplete").

Antigravity is a *code* agent, so this module reads "auto-recomplete" as
**automatic code completion**: given a code context (a prefix, or a file plus a
cursor position), it asks Gemini — keylessly, via aphrody's proven Vertex AI
path — to predict the code that comes next, and returns structured completion
candidates.

Two ways to drive it:

* **single** — :func:`complete` returns N candidates for one context. With
  ``--stream`` the first candidate is streamed token-by-token for a minimal
  time-to-first-token (the user wants latency kept low everywhere).
* **auto / batch** — :func:`recomplete_file` fills a marker in a file in place
  (single-shot), and :func:`recomplete_paths` walks a set of files, completing
  the first marker found in each — the "auto" loop over a project.

The completion uses fill-in-the-middle (FIM) framing when a suffix is present
(code after the cursor), so the model writes only the missing span and keeps the
surrounding code intact. A reused :class:`aphrody.vertex.GeminiVertex` (one
HTTP/genai client) keeps cold-start and per-call latency down.

Nothing here needs an API key: authorization is the on-device Antigravity OAuth
token, exactly like the rest of aphrody.
"""

from __future__ import annotations

import dataclasses
from pathlib import Path
from typing import TYPE_CHECKING, Any

from aphrody.errors import AphrodyError
from aphrody.vertex import DEFAULT_MODEL, GeminiVertex

if TYPE_CHECKING:
    from collections.abc import Iterable, Iterator

#: The default cursor marker recognised in files for in-place completion.
DEFAULT_CURSOR_MARKER = "<|cursor|>"

#: Sentinels Gemini may wrap completions in; stripped from the output.
_FENCE_PREFIXES = ("```",)

#: Map of common file extensions to a language hint for the prompt.
_EXT_LANG = {
    ".py": "python",
    ".pyi": "python",
    ".rs": "rust",
    ".ts": "typescript",
    ".tsx": "tsx",
    ".js": "javascript",
    ".jsx": "jsx",
    ".go": "go",
    ".c": "c",
    ".h": "c",
    ".cc": "cpp",
    ".cpp": "cpp",
    ".hpp": "cpp",
    ".java": "java",
    ".kt": "kotlin",
    ".swift": "swift",
    ".rb": "ruby",
    ".sh": "bash",
    ".sql": "sql",
    ".html": "html",
    ".css": "css",
    ".json": "json",
    ".toml": "toml",
    ".yaml": "yaml",
    ".yml": "yaml",
    ".md": "markdown",
}


class CompletionError(AphrodyError):
    """Raised when a completion request is malformed or fails."""


@dataclasses.dataclass(frozen=True)
class CompletionRequest:
    """A normalised code-completion context.

    Attributes:
        prefix: Code before the cursor (the model continues from here).
        suffix: Code after the cursor; enables fill-in-the-middle when set.
        language: Language hint (e.g. ``"python"``), or ``None``.
        path: Originating file path, for context (never required).
    """

    prefix: str
    suffix: str = ""
    language: str | None = None
    path: str | None = None


@dataclasses.dataclass(frozen=True)
class Completion:
    """A single completion candidate.

    Attributes:
        text: The predicted code span (to insert at the cursor).
        index: Candidate index within the response (0-based).
        model: The model id that produced it.
        finish_reason: The model's finish reason, when reported.
    """

    text: str
    index: int = 0
    model: str = ""
    finish_reason: str | None = None

    def to_dict(self) -> dict[str, Any]:
        """Return a JSON-serialisable view of the candidate."""
        return {
            "text": self.text,
            "index": self.index,
            "model": self.model,
            "finish_reason": self.finish_reason,
        }


def language_for_path(path: str | Path) -> str | None:
    """Infer a language hint from a file extension.

    Args:
        path: A file path.

    Returns:
        A language name, or ``None`` when the extension is unknown.
    """
    return _EXT_LANG.get(Path(path).suffix.lower())


def split_at_cursor(
    text: str,
    *,
    line: int | None = None,
    col: int | None = None,
    marker: str | None = None,
) -> tuple[str, str]:
    """Split ``text`` into ``(prefix, suffix)`` at a cursor location.

    The cursor is located by, in order: an explicit ``marker`` substring (which
    is removed), or a 1-based ``line``/``col`` position. With neither, the
    cursor is the end of the text (pure continuation).

    Args:
        text: The full source text.
        line: 1-based line number of the cursor.
        col: 1-based column on that line (defaults to end of line).
        marker: A marker substring denoting the cursor (e.g. ``<|cursor|>``).

    Returns:
        A ``(prefix, suffix)`` tuple.

    Raises:
        CompletionError: When ``line`` is out of range.
    """
    if marker is not None and marker in text:
        head, _, tail = text.partition(marker)
        return head, tail

    if line is None:
        return text, ""

    lines = text.splitlines(keepends=True)
    if line < 1 or line > len(lines) + 1:
        raise CompletionError(
            f"line {line} is out of range (file has {len(lines)} lines)"
        )

    prefix_lines = lines[: line - 1]
    target = lines[line - 1] if line - 1 < len(lines) else ""
    # Strip a single trailing newline from the target for column maths.
    target_body = target.rstrip("\n")
    if col is None:
        cut = len(target_body)
    else:
        cut = max(0, min(col - 1, len(target_body)))
    newline = target[len(target_body) :]

    prefix = "".join(prefix_lines) + target_body[:cut]
    suffix = target_body[cut:] + newline + "".join(lines[line:])
    return prefix, suffix


def build_prompt(req: CompletionRequest) -> tuple[str, str]:
    """Build a ``(system_instruction, user_prompt)`` pair for a completion.

    Uses fill-in-the-middle framing when a suffix is present; otherwise a plain
    continuation. The model is told to emit only the inserted code — no prose,
    no fences, no restatement of the surrounding context.

    Args:
        req: The completion request.

    Returns:
        A ``(system_instruction, user_prompt)`` tuple.
    """
    lang = req.language or "code"
    system = (
        "You are a code completion engine. Output ONLY the code that should be "
        "inserted at the cursor — no explanations, no markdown fences, and do "
        "not repeat the surrounding code. Preserve the file's existing "
        "indentation and style. If nothing should be inserted, output nothing."
    )
    where = f" from `{req.path}`" if req.path else ""
    if req.suffix.strip():
        user = (
            f"Fill in the missing {lang} code at <CURSOR>{where}.\n"
            f"Return only the text to insert between the prefix and suffix.\n\n"
            f"<PREFIX>\n{req.prefix}\n</PREFIX>\n"
            f"<CURSOR>\n"
            f"<SUFFIX>\n{req.suffix}\n</SUFFIX>"
        )
    else:
        user = (
            f"Continue this {lang} code{where}. Return only the continuation "
            f"to append after the prefix.\n\n"
            f"<PREFIX>\n{req.prefix}\n</PREFIX>"
        )
    return system, user


def _strip_fences(text: str) -> str:
    """Remove a leading/trailing markdown code fence if the model added one."""
    stripped = text.strip("\n")
    if not stripped.startswith(_FENCE_PREFIXES):
        return text
    lines = stripped.splitlines()
    # Drop the opening fence line (``` or ```lang) and a trailing fence.
    if lines and lines[0].startswith("```"):
        lines = lines[1:]
    if lines and lines[-1].strip() == "```":
        lines = lines[:-1]
    return "\n".join(lines)


class CodeCompleter:
    """Keyless code-completion engine over a reused Gemini Vertex client.

    A single :class:`CodeCompleter` reuses one google-genai client across many
    completions (lower per-call latency). Construct with a ``gemini`` to inject
    a client (the tests pass a fake); otherwise one is built lazily on first use
    from the on-device OAuth credentials.
    """

    def __init__(
        self,
        *,
        model: str = DEFAULT_MODEL,
        gemini: GeminiVertex | None = None,
        temperature: float = 0.2,
        max_output_tokens: int = 256,
    ) -> None:
        self._model = model
        self._gemini = gemini
        self._temperature = temperature
        self._max_output_tokens = max_output_tokens

    @property
    def gemini(self) -> GeminiVertex:
        """The lazily-constructed Gemini Vertex client (keyless)."""
        if self._gemini is None:
            self._gemini = GeminiVertex(model=self._model)
        return self._gemini

    def complete(
        self, req: CompletionRequest, *, n: int = 1
    ) -> list[Completion]:
        """Return ``n`` completion candidates for ``req``.

        Args:
            req: The completion context.
            n: Number of candidates to request.

        Returns:
            A list of :class:`Completion` candidates (possibly fewer than ``n``
            if the model returned fewer).
        """
        from google.genai import types as gx

        system, user = build_prompt(req)
        config = gx.GenerateContentConfig(
            system_instruction=system,
            temperature=self._temperature,
            max_output_tokens=self._max_output_tokens,
            candidate_count=n,
        )
        response = self.gemini.client.models.generate_content(
            model=self._model,
            contents=user,
            config=config,
        )
        return self._parse_candidates(response)

    def _parse_candidates(self, response: Any) -> list[Completion]:
        """Turn a google-genai response into :class:`Completion` objects."""
        out: list[Completion] = []
        candidates = getattr(response, "candidates", None) or []
        for i, cand in enumerate(candidates):
            text = _candidate_text(cand)
            if not text:
                continue
            out.append(
                Completion(
                    text=_strip_fences(text),
                    index=i,
                    model=self._model,
                    finish_reason=_finish_reason(cand),
                )
            )
        # Fall back to the convenience ``.text`` accessor for single-candidate
        # responses where ``candidates`` was not populated as expected.
        if not out:
            text = getattr(response, "text", None)
            if text:
                out.append(
                    Completion(
                        text=_strip_fences(text), index=0, model=self._model
                    )
                )
        return out

    def stream(self, req: CompletionRequest) -> Iterator[str]:
        """Yield completion text deltas for the first candidate (low TTFT).

        Args:
            req: The completion context.

        Yields:
            Non-empty text deltas as the model streams them.
        """
        from google.genai import types as gx

        system, user = build_prompt(req)
        config = gx.GenerateContentConfig(
            system_instruction=system,
            temperature=self._temperature,
            max_output_tokens=self._max_output_tokens,
        )
        for chunk in self.gemini.client.models.generate_content_stream(
            model=self._model,
            contents=user,
            config=config,
        ):
            text = getattr(chunk, "text", None)
            if text:
                yield text


def complete(
    *,
    prefix: str | None = None,
    suffix: str = "",
    file: str | None = None,
    line: int | None = None,
    col: int | None = None,
    language: str | None = None,
    marker: str | None = None,
    n: int = 1,
    model: str = DEFAULT_MODEL,
    completer: CodeCompleter | None = None,
) -> list[Completion]:
    """Produce code completions from a prefix or a file+cursor context.

    Exactly one of ``prefix`` or ``file`` must be given.

    Args:
        prefix: Code before the cursor (continuation mode).
        suffix: Code after the cursor (enables fill-in-the-middle).
        file: Path to a source file to complete inside.
        line: 1-based cursor line within ``file`` (or where ``marker`` is).
        col: 1-based cursor column.
        language: Language hint; inferred from ``file`` when omitted.
        marker: Cursor marker substring (default ``<|cursor|>`` for files).
        n: Number of candidates.
        model: Gemini model id.
        completer: An existing :class:`CodeCompleter` to reuse (else built).

    Returns:
        The completion candidates.

    Raises:
        CompletionError: When neither/both of ``prefix``/``file`` are provided.
    """
    if (prefix is None) == (file is None):
        raise CompletionError("provide exactly one of 'prefix' or 'file'")

    if file is not None:
        text = Path(file).read_text(encoding="utf-8")
        eff_marker = marker if marker is not None else DEFAULT_CURSOR_MARKER
        pre, suf = split_at_cursor(text, line=line, col=col, marker=eff_marker)
        req = CompletionRequest(
            prefix=pre,
            suffix=suf,
            language=language or language_for_path(file),
            path=file,
        )
    else:
        req = CompletionRequest(
            prefix=prefix or "",
            suffix=suffix,
            language=language,
        )

    engine = completer or CodeCompleter(model=model)
    return engine.complete(req, n=n)


def recomplete_file(
    file: str,
    *,
    marker: str = DEFAULT_CURSOR_MARKER,
    language: str | None = None,
    model: str = DEFAULT_MODEL,
    completer: CodeCompleter | None = None,
    write: bool = True,
) -> dict[str, Any]:
    """Fill the cursor marker in ``file`` with the top completion, in place.

    The marker substring is replaced by the model's completion. When ``write``
    is false the file is left untouched and the result is returned for preview.

    Args:
        file: Path to the file containing exactly one ``marker``.
        marker: The cursor marker substring to replace.
        language: Language hint; inferred from the path when omitted.
        model: Gemini model id.
        completer: An existing :class:`CodeCompleter` to reuse.
        write: When true, write the completed text back to ``file``.

    Returns:
        A dict describing what was inserted (and whether it was written).

    Raises:
        CompletionError: When the marker is absent.
    """
    path = Path(file)
    text = path.read_text(encoding="utf-8")
    if marker not in text:
        raise CompletionError(f"marker {marker!r} not found in {file}")

    pre, suf = split_at_cursor(text, marker=marker)
    req = CompletionRequest(
        prefix=pre,
        suffix=suf,
        language=language or language_for_path(file),
        path=file,
    )
    engine = completer or CodeCompleter(model=model)
    candidates = engine.complete(req, n=1)
    inserted = candidates[0].text if candidates else ""
    completed = pre + inserted + suf
    if write:
        path.write_text(completed, encoding="utf-8")
    return {
        "file": str(path),
        "inserted": inserted,
        "written": bool(write),
        "model": candidates[0].model if candidates else model,
    }


def recomplete_paths(
    paths: Iterable[str],
    *,
    marker: str = DEFAULT_CURSOR_MARKER,
    model: str = DEFAULT_MODEL,
    write: bool = True,
    completer: CodeCompleter | None = None,
) -> list[dict[str, Any]]:
    """Auto-complete the marker in each of ``paths`` (the batch / auto loop).

    A single :class:`CodeCompleter` is reused across all files to keep the
    google-genai client warm. Files without the marker are reported as skipped
    rather than failing the whole run.

    Args:
        paths: File paths to process.
        marker: Cursor marker substring.
        model: Gemini model id.
        write: Whether to write completions back to disk.
        completer: An existing :class:`CodeCompleter` to reuse.

    Returns:
        One result dict per input path.
    """
    engine = completer or CodeCompleter(model=model)
    results: list[dict[str, Any]] = []
    for p in paths:
        try:
            results.append(
                recomplete_file(
                    p,
                    marker=marker,
                    model=model,
                    completer=engine,
                    write=write,
                )
            )
        except CompletionError as exc:
            results.append({"file": str(p), "skipped": str(exc)})
        except OSError as exc:
            results.append({"file": str(p), "error": str(exc)})
    return results


def _candidate_text(cand: Any) -> str:
    """Concatenate the text parts of a google-genai candidate."""
    content = getattr(cand, "content", None)
    parts = getattr(content, "parts", None) or []
    chunks: list[str] = []
    for part in parts:
        text = getattr(part, "text", None)
        if text:
            chunks.append(text)
    return "".join(chunks)


def _finish_reason(cand: Any) -> str | None:
    """Extract a stringified finish reason from a candidate, if any."""
    reason = getattr(cand, "finish_reason", None)
    if reason is None:
        return None
    return getattr(reason, "name", None) or str(reason)
