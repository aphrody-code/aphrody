# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Run the ``google-antigravity`` SDK against aphrody's keyless backend.

The Antigravity SDK normally talks to a **local Go harness** (the ``agy``
binary): :class:`google.antigravity.connections.local.LocalConnection` spawns
``localharness``, drives it over a length-prefixed protobuf handshake, and
streams ``StepUpdate`` events over a WebSocket. That harness, in turn, holds a
Gemini API key.

This module provides a drop-in :class:`~google.antigravity.connections.connection.Connection`
that **needs no harness, no agy binary, and no API key**. It satisfies the SDK
contract (``Connection`` / ``ConnectionStrategy`` / ``AgentConfig``) by
delegating every turn to :class:`aphrody.vertex.GeminiVertex` — aphrody's proven
keyless path: the on-device Antigravity OAuth token (read from the OS credential
store, refreshed transparently) authorises Gemini on Vertex AI directly.

Approach (documented for reviewers)
------------------------------------
The native ``Connection`` contract is heavily coupled to the Go harness wire
format (``localharness_pb2``: tool confirmations, subagent trajectories,
question requests, file-edit diffs). Re-implementing that protobuf surface on
top of a stateless HTTP backend would be a large, fragile translation layer for
little gain — the keyless backend does not run host-side built-in tools the way
the harness does.

So we implement the **clean, public** ``Connection`` interface directly
(``send`` / ``receive_steps`` / ``send_trigger_notification`` + the optional
lifecycle hooks) and emit the SDK's own ``types.Step`` objects. This makes the
whole Layer-2 (``Conversation``) and Layer-1 (``Agent``) surface — including
``Agent.chat()`` streaming, ``conversation.history``, ``last_response`` and
token usage — work unchanged against aphrody's keyless Gemini.

Custom Python tools, MCP servers and hooks declared on the config are still
wired by the SDK's ``Agent`` (``ToolRunner`` / ``HookRunner`` /
``McpBridge``); this connection exposes them to the model as Gemini function
declarations and executes the model's function calls through the SDK
``ToolRunner``, so a tool-using agent works end to end.

Usage
-----
    >>> import asyncio
    >>> from google.antigravity import Agent
    >>> from aphrody.antigravity_bridge import AphrodyAgentConfig
    >>>
    >>> async def main():
    ...     async with Agent(AphrodyAgentConfig(model="gemini-2.5-flash")) as a:
    ...         resp = await a.chat("Say hello in one word.")
    ...         print(await resp.text())
    >>> asyncio.run(main())  # doctest: +SKIP

or, without the Agent layer, drive a :class:`Conversation` directly:

    >>> from google.antigravity.conversation import Conversation
    >>> from aphrody.antigravity_bridge import AphrodyConnectionStrategy
    >>> async def chat():  # doctest: +SKIP
    ...     async with Conversation.create(AphrodyConnectionStrategy()) as conv:
    ...         await (await conv.chat("Hi")).text()
"""

from __future__ import annotations

import asyncio
import json
from typing import TYPE_CHECKING, Any

from aphrody.vertex import DEFAULT_MODEL, GeminiVertex

if TYPE_CHECKING:
    from collections.abc import AsyncIterator, Callable, Iterator

    from google.oauth2.credentials import Credentials

# The SDK is an optional peer (workspace dep); import lazily so that importing
# :mod:`aphrody` never hard-requires ``google-antigravity`` to be installed.
try:  # pragma: no cover - exercised indirectly by the bridge tests
    from google.antigravity import types as _agtypes
    from google.antigravity.connections import connection as _agconnection

    _SDK_AVAILABLE = True
except ImportError:  # pragma: no cover - environments without the SDK
    _agtypes = None  # type: ignore[assignment]
    _agconnection = None  # type: ignore[assignment]
    _SDK_AVAILABLE = False


class BridgeUnavailableError(RuntimeError):
    """Raised when the ``google-antigravity`` SDK is not importable."""


def _require_sdk() -> None:
    """Fail loudly (not at import time) when the SDK is missing."""
    if not _SDK_AVAILABLE:
        raise BridgeUnavailableError(
            "The 'google-antigravity' SDK is not installed. Install it (it is a"
            " workspace member: `uv sync`) to use the aphrody Antigravity"
            " bridge."
        )


# A sentinel pushed onto the step queue to mark end-of-turn.
_TURN_DONE = object()


def _content_to_genai(prompt: Any) -> Any:
    """Convert an SDK ``types.Content`` prompt into google-genai ``contents``.

    The SDK ``Content`` is ``str | media | list[...]``. google-genai accepts a
    plain string, a list of parts, or multi-turn message lists. Strings and
    string lists pass through directly; semantic media (Image/Document/Audio/
    Video) are mapped to google-genai ``Part`` blobs.

    Args:
        prompt: The SDK prompt (``types.Content`` or ``None``).

    Returns:
        A value suitable for google-genai ``contents``.
    """
    if prompt is None:
        return ""
    if isinstance(prompt, str):
        return prompt

    items = prompt if isinstance(prompt, list) else [prompt]
    from google.genai import types as gx

    parts: list[Any] = []
    for item in items:
        if isinstance(item, str):
            parts.append(gx.Part.from_text(text=item))
        elif _is_media(item):
            parts.append(
                gx.Part.from_bytes(data=item.data, mime_type=item.mime_type)
            )
        else:
            parts.append(gx.Part.from_text(text=str(item)))
    return parts


def _is_media(item: Any) -> bool:
    """Return True for an SDK semantic-media content primitive."""
    if not _SDK_AVAILABLE:
        return False
    return isinstance(
        item,
        (_agtypes.Image, _agtypes.Document, _agtypes.Audio, _agtypes.Video),
    )


def _tools_as_genai(tool_runner: Any) -> Any | None:
    """Expose the SDK ToolRunner's callables to Gemini as function tools.

    Args:
        tool_runner: The SDK ``ToolRunner`` (or ``None``).

    Returns:
        A ``google.genai`` ``Tool`` wrapping the public callables, or ``None``
        when there are no tools to expose.
    """
    if tool_runner is None or not getattr(tool_runner, "tools", None):
        return None

    from google.genai import types as gx

    declarations: list[Any] = []
    for name in tool_runner.tools:
        fn = tool_runner.get_public_callable(name)
        try:
            decl = gx.FunctionDeclaration.from_callable_with_api_option(
                callable=fn, api_option="VERTEX_AI"
            )
        except Exception:  # pylint: disable=broad-except
            # A callable whose signature google-genai cannot introspect is
            # skipped rather than aborting the whole turn.
            continue
        declarations.append(decl)

    if not declarations:
        return None
    return gx.Tool(function_declarations=declarations)


if _SDK_AVAILABLE:

    class AphrodyConnection(_agconnection.Connection):
        """A keyless Antigravity :class:`Connection` backed by Vertex Gemini.

        Each :meth:`send` issues a streaming ``generate_content`` against
        Vertex AI (via :class:`aphrody.vertex.GeminiVertex`) using the on-device
        OAuth token, and surfaces the model output as SDK ``Step`` objects.

        The connection keeps a multi-turn ``contents`` history so that
        ``Conversation.chat()`` behaves like a real session. When custom tools
        are configured, the model's function calls are executed through the SDK
        ``ToolRunner`` and fed back to the model until it produces a final text
        answer.
        """

        def __init__(
            self,
            *,
            gemini: GeminiVertex,
            tool_runner: Any = None,
            hook_runner: Any = None,
            system_instruction: str | None = None,
            conversation_id: str = "",
            max_tool_iterations: int = 8,
        ) -> None:
            self._gemini = gemini
            self._tool_runner = tool_runner
            self._hook_runner = hook_runner
            self._system_instruction = system_instruction
            self._conversation_id = conversation_id
            self._max_tool_iterations = max_tool_iterations

            self._history: list[Any] = []
            self._step_queue: asyncio.Queue[Any] = asyncio.Queue()
            self._idle = asyncio.Event()
            self._idle.set()
            self._step_index = 0
            self._current_turn_context: Any = None
            self._is_receiving = False

        # -- introspection ----------------------------------------------------

        @property
        def is_idle(self) -> bool:
            """True when no turn is in flight."""
            return self._idle.is_set()

        @property
        def conversation_id(self) -> str:
            """The (best-effort) conversation identifier."""
            return self._conversation_id

        # -- helpers ----------------------------------------------------------

        def _next_index(self) -> int:
            self._step_index += 1
            return self._step_index

        def _text_step(
            self,
            *,
            delta: str = "",
            content: str = "",
            status: Any = None,
            complete: bool = False,
            usage: Any = None,
        ) -> Any:
            """Build a MODEL→USER text Step (delta or terminal)."""
            return _agtypes.Step(
                id=f"{self._conversation_id}:{self._step_index}",
                step_index=self._step_index,
                type=_agtypes.StepType.TEXT_RESPONSE,
                source=_agtypes.StepSource.MODEL,
                target=_agtypes.StepTarget.USER,
                status=status or _agtypes.StepStatus.ACTIVE,
                content=content,
                content_delta=delta,
                is_complete_response=complete,
                usage_metadata=usage,
            )

        # -- the streaming engine --------------------------------------------

        async def _run_turn(self, contents: Any) -> None:
            """Stream one model turn into the step queue, handling tool calls.

            Runs the blocking google-genai streaming call in a worker thread and
            bridges its chunks back onto the event loop via a thread-safe queue.
            """
            loop = asyncio.get_running_loop()
            self._history.append({"role": "user", "parts": _as_parts(contents)})

            tool = _tools_as_genai(self._tool_runner)

            for _ in range(self._max_tool_iterations):
                chunk_q: asyncio.Queue[Any] = asyncio.Queue()

                def _produce(history: list[Any]) -> None:
                    try:
                        for piece in self._stream_once(history, tool):
                            loop.call_soon_threadsafe(chunk_q.put_nowait, piece)
                    except Exception as exc:  # pylint: disable=broad-except
                        loop.call_soon_threadsafe(chunk_q.put_nowait, exc)
                    finally:
                        loop.call_soon_threadsafe(
                            chunk_q.put_nowait, _TURN_DONE
                        )

                worker = loop.run_in_executor(
                    None, _produce, list(self._history)
                )

                text_acc = ""
                function_calls: list[Any] = []
                usage: Any = None
                idx = self._next_index()
                while True:
                    piece = await chunk_q.get()
                    if piece is _TURN_DONE:
                        break
                    if isinstance(piece, Exception):
                        await worker
                        raise piece
                    kind, payload = piece
                    if kind == "text":
                        text_acc += payload
                        await self._step_queue.put(
                            self._text_step(delta=payload)
                        )
                    elif kind == "function_call":
                        function_calls.append(payload)
                    elif kind == "usage":
                        usage = payload
                await worker

                if not function_calls:
                    # Final answer for this turn.
                    self._history.append(
                        {"role": "model", "parts": [{"text": text_acc}]}
                    )
                    await self._step_queue.put(
                        self._text_step(
                            content=text_acc,
                            status=_agtypes.StepStatus.DONE,
                            complete=True,
                            usage=usage,
                        )
                    )
                    return

                # Record the model's function-call turn, run the tools, append
                # the results, and loop so the model can use them.
                self._history.append(
                    {
                        "role": "model",
                        "parts": _function_calls_to_parts(function_calls),
                    }
                )
                await self._emit_tool_calls(idx, function_calls)
                results = await self._execute_tools(function_calls)
                self._history.append(
                    {"role": "user", "parts": _tool_results_to_parts(results)}
                )

            # Exhausted the tool-iteration budget without a final text answer.
            await self._step_queue.put(
                self._text_step(
                    content="",
                    status=_agtypes.StepStatus.DONE,
                    complete=True,
                )
            )

        def _stream_once(
            self, history: list[Any], tool: Any
        ) -> Iterator[tuple[str, Any]]:
            """Blocking generator of ``(kind, payload)`` chunks for one call.

            Yields ``("text", str)`` deltas, ``("function_call", fc)`` for each
            model function call, and a final ``("usage", UsageMetadata)``. Runs
            in a worker thread.
            """
            from google.genai import types as gx

            config = None
            if self._system_instruction or tool is not None:
                config = gx.GenerateContentConfig(
                    system_instruction=self._system_instruction,
                    tools=[tool] if tool is not None else None,
                )

            last_usage: Any = None
            for chunk in self._gemini.client.models.generate_content_stream(
                model=self._gemini.model,
                contents=history,
                config=config,
            ):
                text = getattr(chunk, "text", None)
                if text:
                    yield ("text", text)
                for fc in _iter_function_calls(chunk):
                    yield ("function_call", fc)
                last_usage = _extract_usage(chunk) or last_usage
            if last_usage is not None:
                yield ("usage", last_usage)

        async def _emit_tool_calls(
            self, idx: int, function_calls: list[Any]
        ) -> None:
            """Push a MODEL→ENVIRONMENT TOOL_CALL step for visibility."""
            calls = [
                _agtypes.ToolCall(
                    id=f"{self._conversation_id}:{idx}:{i}",
                    name=getattr(fc, "name", "") or "",
                    args=dict(getattr(fc, "args", {}) or {}),
                )
                for i, fc in enumerate(function_calls)
            ]
            await self._step_queue.put(
                _agtypes.Step(
                    id=f"{self._conversation_id}:{idx}",
                    step_index=idx,
                    type=_agtypes.StepType.TOOL_CALL,
                    source=_agtypes.StepSource.MODEL,
                    target=_agtypes.StepTarget.ENVIRONMENT,
                    status=_agtypes.StepStatus.ACTIVE,
                    tool_calls=calls,
                )
            )

        async def _execute_tools(self, function_calls: list[Any]) -> list[Any]:
            """Run model function calls through the SDK ToolRunner.

            Returns a list of ``(name, result_obj_or_error)`` tuples used to
            build the function-response parts fed back to the model.
            """
            results: list[tuple[str, Any]] = []
            for fc in function_calls:
                name = getattr(fc, "name", "") or ""
                args = dict(getattr(fc, "args", {}) or {})
                if self._tool_runner is None:
                    results.append(
                        (name, {"error": "no tool runner configured"})
                    )
                    continue
                try:
                    runner_results = await self._tool_runner.process_tool_calls(
                        [_agtypes.ToolCall(name=name, args=args)]
                    )
                    res = runner_results[0]
                    if res.error:
                        results.append((name, {"error": res.error}))
                    else:
                        results.append((name, _jsonable(res.result)))
                except Exception as exc:  # pylint: disable=broad-except
                    results.append((name, {"error": repr(exc)}))
            return results

        # -- Connection contract ---------------------------------------------

        async def send(self, prompt: Any, **kwargs: Any) -> None:
            """Send a prompt and run one keyless model turn into the queue."""
            self._idle.clear()
            if self._hook_runner is not None:
                res, turn_context = await self._hook_runner.dispatch_pre_turn(
                    prompt
                )
                self._current_turn_context = turn_context
                if not res.allow:
                    await self._step_queue.put(
                        _agtypes.Step(
                            type=_agtypes.StepType.SYSTEM_MESSAGE,
                            source=_agtypes.StepSource.SYSTEM,
                            status=_agtypes.StepStatus.CANCELED,
                            error=res.message or "Turn denied by hook.",
                        )
                    )
                    await self._step_queue.put(_TURN_DONE)
                    self._idle.set()
                    return

            contents = _content_to_genai(prompt)
            try:
                await self._run_turn(contents)
            except Exception as exc:  # pylint: disable=broad-except
                await self._step_queue.put(
                    _agtypes.Step(
                        type=_agtypes.StepType.SYSTEM_MESSAGE,
                        source=_agtypes.StepSource.SYSTEM,
                        status=_agtypes.StepStatus.ERROR,
                        error=str(exc),
                    )
                )
            finally:
                await self._step_queue.put(_TURN_DONE)
                self._idle.set()

        async def receive_steps(self) -> AsyncIterator[Any]:  # noqa: D102
            if self._is_receiving:
                raise RuntimeError(
                    "Concurrent receive_steps() calls are not supported."
                )
            self._is_receiving = True
            try:
                while True:
                    step = await self._step_queue.get()
                    if step is _TURN_DONE:
                        if self._hook_runner and self._current_turn_context:
                            await self._hook_runner.dispatch_post_turn(
                                self._current_turn_context,
                                "",
                            )
                            self._current_turn_context = None
                        return
                    yield step
            finally:
                self._is_receiving = False

        async def wait_for_idle(self) -> None:  # noqa: D102 - inherited
            await self._idle.wait()

        async def send_trigger_notification(self, content: str) -> None:
            """Deliver a trigger to the model as an automated user message."""
            await self.send(content)

        async def disconnect(self) -> None:  # noqa: D102 - inherited
            if self._hook_runner and getattr(
                self._hook_runner, "on_session_end_hooks", None
            ):
                await self._hook_runner.dispatch_session_end()
            self._gemini = None  # type: ignore[assignment]

    class AphrodyConnectionStrategy(_agconnection.ConnectionStrategy):
        """Establishes an :class:`AphrodyConnection` (no harness, no agy)."""

        def __init__(
            self,
            *,
            model: str = DEFAULT_MODEL,
            project: str | None = None,
            location: str | None = None,
            credentials: Credentials | None = None,
            tool_runner: Any = None,
            hook_runner: Any = None,
            system_instruction: str | None = None,
            conversation_id: str | None = None,
        ) -> None:
            _require_sdk()
            self._model = model
            self._project = project
            self._location = location
            self._credentials = credentials
            self._tool_runner = tool_runner
            self._hook_runner = hook_runner
            self._system_instruction = system_instruction
            self._conversation_id = conversation_id or ""
            self._connection: AphrodyConnection | None = None

        def connect(self) -> Any:  # noqa: D102 - inherited
            if self._connection is None:
                raise RuntimeError(
                    "Connection not established. Use as a context manager."
                )
            return self._connection

        async def __aenter__(self) -> None:
            gemini = GeminiVertex(
                project=self._project,
                location=self._location,
                model=self._model,
                creds=self._credentials,
            )
            self._connection = AphrodyConnection(
                gemini=gemini,
                tool_runner=self._tool_runner,
                hook_runner=self._hook_runner,
                system_instruction=_coerce_system_instruction(
                    self._system_instruction
                ),
                conversation_id=self._conversation_id,
            )
            if self._hook_runner and getattr(
                self._hook_runner, "on_session_start_hooks", None
            ):
                await self._hook_runner.dispatch_session_start()

        async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
            if self._connection is not None:
                await self._connection.disconnect()
                self._connection = None

    class AphrodyAgentConfig(_agconnection.AgentConfig):
        """SDK ``AgentConfig`` that runs on aphrody's keyless Vertex backend.

        Pass this to :class:`google.antigravity.Agent` exactly like the local
        config, but it never spawns the Go harness and never needs an API key.

        Attributes:
            model: The Gemini model id (defaults to aphrody's
                :data:`~aphrody.vertex.DEFAULT_MODEL`).
            project: Optional Vertex project override.
            location: Optional Vertex location override.
        """

        model: str = DEFAULT_MODEL
        project: str | None = None
        location: str | None = None

        def create_strategy(self, *, tool_runner: Any, hook_runner: Any) -> Any:
            """Build the keyless strategy with the SDK-wired runners."""
            return AphrodyConnectionStrategy(
                model=self.model,
                project=self.project,
                location=self.location,
                tool_runner=tool_runner,
                hook_runner=hook_runner,
                system_instruction=_coerce_system_instruction(
                    self.system_instructions
                ),
                conversation_id=self.conversation_id,
            )


def _coerce_system_instruction(value: Any) -> str | None:
    """Flatten an SDK ``SystemInstructions`` (or str) into plain text."""
    if value is None or isinstance(value, str):
        return value
    # TemplatedSystemInstructions / CustomSystemInstructions.
    text = getattr(value, "text", None)
    if isinstance(text, str):
        return text
    sections = getattr(value, "sections", None)
    if sections:
        return "\n\n".join(
            getattr(s, "content", "")
            for s in sections
            if getattr(s, "content", "")
        )
    return None


def _as_parts(contents: Any) -> list[dict[str, Any]]:
    """Normalise google-genai ``contents`` into a list of part dicts."""
    if isinstance(contents, str):
        return [{"text": contents}]
    parts: list[dict[str, Any]] = []
    for part in contents if isinstance(contents, list) else [contents]:
        text = getattr(part, "text", None)
        if text is not None:
            parts.append({"text": text})
        else:
            parts.append({"text": str(part)})
    return parts


def _iter_function_calls(chunk: Any) -> list[Any]:
    """Extract function calls from a streaming chunk (best effort)."""
    calls = getattr(chunk, "function_calls", None)
    if calls:
        return list(calls)
    out: list[Any] = []
    for cand in getattr(chunk, "candidates", None) or []:
        content = getattr(cand, "content", None)
        for part in getattr(content, "parts", None) or []:
            fc = getattr(part, "function_call", None)
            if fc is not None:
                out.append(fc)
    return out


def _extract_usage(chunk: Any) -> Any | None:
    """Map a google-genai ``usage_metadata`` onto the SDK ``UsageMetadata``."""
    um = getattr(chunk, "usage_metadata", None)
    if um is None or not _SDK_AVAILABLE:
        return None
    return _agtypes.UsageMetadata(
        prompt_token_count=getattr(um, "prompt_token_count", None),
        cached_content_token_count=getattr(
            um, "cached_content_token_count", None
        ),
        candidates_token_count=getattr(um, "candidates_token_count", None),
        thoughts_token_count=getattr(um, "thoughts_token_count", None),
        total_token_count=getattr(um, "total_token_count", None),
    )


def _function_calls_to_parts(function_calls: list[Any]) -> list[dict[str, Any]]:
    """Build google-genai ``functionCall`` parts for the history."""
    return [
        {
            "function_call": {
                "name": getattr(fc, "name", "") or "",
                "args": dict(getattr(fc, "args", {}) or {}),
            }
        }
        for fc in function_calls
    ]


def _tool_results_to_parts(
    results: list[tuple[str, Any]],
) -> list[dict[str, Any]]:
    """Build google-genai ``functionResponse`` parts for the history."""
    parts: list[dict[str, Any]] = []
    for name, payload in results:
        response = payload if isinstance(payload, dict) else {"result": payload}
        parts.append(
            {"function_response": {"name": name, "response": response}}
        )
    return parts


def _jsonable(value: Any) -> Any:
    """Coerce a tool result into a JSON-serialisable value."""
    if value is None or isinstance(value, (str, int, float, bool, dict, list)):
        return value
    for attr in ("model_dump", "dict"):
        method = getattr(value, attr, None)
        if callable(method):
            try:
                return method()
            except Exception:  # pylint: disable=broad-except
                break
    try:
        json.dumps(value)
        return value
    except (TypeError, ValueError):
        return str(value)


def aphrody_agent(
    *,
    model: str = DEFAULT_MODEL,
    system_instruction: str | None = None,
    project: str | None = None,
    location: str | None = None,
    tools: list[Callable[..., Any]] | None = None,
    **config_kwargs: Any,
) -> Any:
    """Build a ``google.antigravity.Agent`` bound to aphrody's keyless backend.

    This is the recommended one-call entry point. The returned ``Agent`` is an
    async context manager:

        >>> async with aphrody_agent(model="gemini-2.5-flash") as agent:  # doctest: +SKIP
        ...     resp = await agent.chat("Hello")
        ...     print(await resp.text())

    Args:
        model: Gemini model id.
        system_instruction: Optional system prompt.
        project: Optional Vertex project override.
        location: Optional Vertex location override.
        tools: Optional list of Python callables exposed to the model as tools.
        **config_kwargs: Forwarded to :class:`AphrodyAgentConfig` (e.g.
            ``policies``, ``hooks``, ``mcp_servers``, ``conversation_id``).

    Returns:
        A configured :class:`google.antigravity.Agent`.
    """
    _require_sdk()
    from google.antigravity import Agent

    config = AphrodyAgentConfig(
        model=model,
        project=project,
        location=location,
        system_instructions=system_instruction,
        tools=tools or [],
        **config_kwargs,
    )
    return Agent(config)
