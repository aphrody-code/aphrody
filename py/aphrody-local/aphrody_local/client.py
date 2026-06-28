"""OpenAI-compatible client wrapper for local engines."""

from __future__ import annotations

from collections.abc import Iterator

from openai import OpenAI

from .config import Settings
from .engines import probe_models, resolve_base_url

# Small, fast models to prefer when the caller does not pin one.
_PREFERRED = ("llama3.2:1b", "qwen2.5:3b", "gemma4:12b")


class LocalAI:
    """A thin, local-first wrapper over the OpenAI SDK."""

    def __init__(self, settings: Settings | None = None) -> None:
        self.settings = settings or Settings.from_env()
        self.base_url = resolve_base_url(self.settings.base_url)
        self._client = OpenAI(
            base_url=self.base_url,
            api_key=self.settings.api_key or "local",
            timeout=self.settings.timeout,
        )

    def list_models(self) -> list[str]:
        """List model ids advertised by the resolved engine."""
        return probe_models(self.base_url)

    def pick_model(self, model: str | None = None) -> str:
        """Resolve the effective model: arg > settings > preferred > first."""
        if model:
            return model
        if self.settings.model:
            return self.settings.model
        models = self.list_models()
        if not models:
            raise RuntimeError(
                f"no models available at {self.base_url}; is the engine running?"
            )
        for pref in _PREFERRED:
            if pref in models:
                return pref
        return models[0]

    def chat_stream(
        self,
        prompt: str,
        *,
        model: str | None = None,
        system: str | None = None,
    ) -> Iterator[str]:
        """Yield assistant text deltas as they stream in."""
        messages: list[dict[str, str]] = []
        if system:
            messages.append({"role": "system", "content": system})
        messages.append({"role": "user", "content": prompt})
        stream = self._client.chat.completions.create(
            model=self.pick_model(model), messages=messages, stream=True
        )
        for chunk in stream:
            if not chunk.choices:
                continue
            delta = chunk.choices[0].delta.content
            if delta:
                yield delta

    def chat(
        self,
        prompt: str,
        *,
        model: str | None = None,
        system: str | None = None,
    ) -> str:
        """Return the full assistant reply (non-streamed convenience)."""
        return "".join(self.chat_stream(prompt, model=model, system=system))

    def embed(self, texts: list[str]) -> list[list[float]]:
        """Embed texts via the engine's ``/v1/embeddings`` (needs an embed model).

        Note: Ollama needs an embedding model pulled (``ollama pull
        nomic-embed-text``). For a torch-free path use the local RAG backend.
        """
        resp = self._client.embeddings.create(
            model=self.settings.embed_model, input=texts
        )
        return [d.embedding for d in resp.data]
