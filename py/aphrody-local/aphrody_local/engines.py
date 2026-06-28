"""Discovery of local OpenAI-compatible inference engines."""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from dataclasses import dataclass


@dataclass(frozen=True)
class Engine:
    """A local OpenAI-compatible engine endpoint."""

    name: str
    base_url: str  # the OpenAI ``/v1`` base


# Probe order: aphrody-serve first (the unifying layer), then raw engines.
KNOWN_ENGINES: tuple[Engine, ...] = (
    Engine("aphrody-serve", "http://127.0.0.1:8088/v1"),
    Engine("ollama", "http://127.0.0.1:11434/v1"),
    Engine("vllm", "http://127.0.0.1:8000/v1"),
    Engine("llamacpp", "http://127.0.0.1:8080/v1"),
)


def probe_models(base_url: str, timeout: float = 2.0) -> list[str]:
    """Return model ids if ``{base}/models`` answers, else an empty list."""
    url = base_url.rstrip("/") + "/models"
    try:
        with urllib.request.urlopen(url, timeout=timeout) as resp:  # noqa: S310
            payload = json.loads(resp.read().decode())
    except (urllib.error.URLError, OSError, ValueError, TimeoutError):
        return []
    return [m.get("id") for m in payload.get("data", []) if m.get("id")]


def discover() -> list[tuple[Engine, list[str]]]:
    """Return every reachable engine with its advertised models."""
    found: list[tuple[Engine, list[str]]] = []
    for engine in KNOWN_ENGINES:
        models = probe_models(engine.base_url)
        if models:
            found.append((engine, models))
    return found


def resolve_base_url(explicit: str | None = None) -> str:
    """Pick the base URL: explicit override, else first live engine, else Ollama."""
    if explicit:
        return explicit
    found = discover()
    if found:
        return found[0][0].base_url
    return "http://127.0.0.1:11434/v1"
