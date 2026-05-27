# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Direct connection to the Gemini *web app* (``gemini.google.com``).

This is the cookie-authenticated path: it speaks the same Boq ``batchexecute``
protocol the web UI uses, so the CLI gets exactly what the browser gets — the
consumer Gemini app, **with no API key and no OAuth token**, only the user's
Google session cookies (managed by :mod:`aphrody.auth.cookies`).

Flow (mirrors the browser):

1. ``GET /app`` with the session cookies, scrape ``SNlM0e`` (the per-session
   ``at`` anti-forgery token) and ``cfb2h`` (the ``bl`` build label) out of
   ``WIZ_global_data`` — the same handshake validated for the in-tree
   NotebookLM Boq client.
2. ``POST .../assistant.lamda.BardFrontendService/StreamGenerate`` with an
   ``f.req`` batchexecute envelope carrying the prompt (and, for follow-ups,
   the conversation ids).
3. Parse the chunked ``)]}'``-guarded response, pull the ``wrb.fr`` envelope,
   and decode the model's reply plus the conversation ids for continuation.

The :class:`GeminiWebClient` keeps conversation state, so repeated
:meth:`~GeminiWebClient.generate` calls form a single threaded chat.
"""

from __future__ import annotations

import json
import os
import random
import re
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    import httpx

from aphrody.auth import cookies
from aphrody.errors import AphrodyError

#: Gemini web host and the StreamGenerate RPC path.
GEMINI_HOST = "https://gemini.google.com"
GEMINI_APP_URL = f"{GEMINI_HOST}/app"
STREAM_GENERATE_PATH = (
    "/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
)

#: Host used for cookie-header construction.
GEMINI_COOKIE_HOST = "gemini.google.com"

#: A recent Chrome desktop User-Agent (the endpoint is UA-sensitive).
DEFAULT_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
)

#: Fallback build label used only if ``cfb2h`` is absent from the page.
DEFAULT_BL = "boq_assistant-bard-web-server_20240625.13_p0"

_SNLM0E_RE = re.compile(r'"SNlM0e":"([^"]+)"')
_CFB2H_RE = re.compile(r'"cfb2h":"([^"]+)"')


class GeminiWebError(AphrodyError):
    """A failure talking to the Gemini web (Boq) endpoint."""


class GeminiWebClient:
    """A cookie-authenticated client for the Gemini web app.

    Example:
        >>> client = GeminiWebClient()
        >>> client.generate("Hello, who are you?")  # doctest: +SKIP
        'I am Gemini, ...'
    """

    def __init__(
        self,
        *,
        model: str = "flash",
        jar: cookies.CookieJar | None = None,
        http: httpx.Client | None = None,
        user_agent: str = DEFAULT_USER_AGENT,
        client_uuid: str = "00000000-0000-4000-8000-000000000001",
    ) -> None:
        """Initialize the client.

        Args:
            model: The default Gemini model to use ("flash-lite", "flash", "pro").
            jar: A cookie jar; loaded from the private store when omitted.
            http: An ``httpx.Client`` to reuse; one is created when omitted.
            user_agent: The browser User-Agent to present.
            client_uuid: A stable UUID to identify the client.

        Raises:
            CookieStoreError: The required session cookie is missing.
        """
        self._jar = jar or cookies.load()
        self._jar.require()  # at minimum __Secure-1PSID
        self._ua = user_agent
        if http is None:
            import httpx

            self._http = httpx.Client(timeout=60.0, follow_redirects=True)
        else:
            self._http = http
        self._owns_http = http is None

        self.model = model
        self.client_uuid = client_uuid
        self._at: str | None = None
        self._bl: str | None = None
        # Conversation continuation ids (cid, rid, rcid).
        self._cid: str | None = None
        self._rid: str | None = None
        self._rcid: str | None = None
        self.last_title: str | None = None

    # -- lifecycle -----------------------------------------------------------

    def close(self) -> None:
        """Close the underlying HTTP client if owned by this instance."""
        if self._owns_http:
            self._http.close()

    def __enter__(self) -> GeminiWebClient:
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    def reset(self) -> None:
        """Forget the current conversation (next call starts a new thread)."""
        self._cid = self._rid = self._rcid = None
        self.last_title = None

    def resume(
        self, cid: str, rid: str | None = None, rcid: str | None = None
    ) -> None:
        """Resume a conversation with the given IDs."""
        self._cid = cid
        self._rid = rid
        self._rcid = rcid

    @property
    def conversation(self) -> tuple[str | None, str | None, str | None]:
        """The current ``(cid, rid, rcid)`` conversation ids."""
        return (self._cid, self._rid, self._rcid)

    def list_conversations(self) -> list[dict[str, Any]]:
        """List recent conversations.

        Returns:
            A list of dicts: ``[{"cid": "c_...", "title": "...", "updated_at": ...}]``.
        """
        import httpx

        if self._at is None:
            self.bootstrap()

        # The payload for MaZiqc: [limit, None, [0, None, 1]]
        inner_args = json.dumps([20, None, [0, None, 1]])
        f_req = [[["MaZiqc", inner_args, None, "generic"]]]

        params = {
            "bl": self._bl or DEFAULT_BL,
            "_reqid": str(random.randint(100000, 999999)),
            "rt": "c",
        }
        form = {"at": self._at, "f.req": json.dumps(f_req)}
        headers = {
            **self._base_headers(),
            "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
            "Origin": GEMINI_HOST,
            "Referer": GEMINI_APP_URL,
            "X-Same-Domain": "1",
        }

        try:
            resp = self._http.post(
                GEMINI_HOST + "/_/BardChatUi/data/batchexecute",
                params=params,
                data=form,
                headers=headers,
            )
        except httpx.HTTPError as exc:
            raise GeminiWebError(
                f"batchexecute request for MaZiqc failed: {exc}"
            ) from exc

        if resp.status_code != 200:
            if resp.status_code in (400, 401, 403):
                self.bootstrap()
                headers["Cookie"] = self._jar.header(GEMINI_COOKIE_HOST)
                form["at"] = self._at
                resp = self._http.post(
                    GEMINI_HOST + "/_/BardChatUi/data/batchexecute",
                    params=params,
                    data=form,
                    headers=headers,
                )
            if resp.status_code != 200:
                raise GeminiWebError(
                    f"batchexecute returned HTTP {resp.status_code}"
                )

        return self._parse_conversations(resp.text)

    def delete_conversation(self, cid: str) -> None:
        """Delete a conversation by ID.

        Args:
            cid: The conversation ID starting with ``c_``.
        """
        import httpx

        if self._at is None:
            self.bootstrap()

        # Payload for GzXR5e: [cid, 1]
        inner_args = json.dumps([cid, 1])
        f_req = [[["GzXR5e", inner_args, None, "generic"]]]

        params = {
            "bl": self._bl or DEFAULT_BL,
            "_reqid": str(random.randint(100000, 999999)),
            "rt": "c",
        }
        form = {"at": self._at, "f.req": json.dumps(f_req)}
        headers = {
            **self._base_headers(),
            "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
            "Origin": GEMINI_HOST,
            "Referer": GEMINI_APP_URL,
            "X-Same-Domain": "1",
        }

        try:
            resp = self._http.post(
                GEMINI_HOST + "/_/BardChatUi/data/batchexecute",
                params=params,
                data=form,
                headers=headers,
            )
        except httpx.HTTPError as exc:
            raise GeminiWebError(
                f"batchexecute request for GzXR5e failed: {exc}"
            ) from exc

        if resp.status_code != 200:
            if resp.status_code in (400, 401, 403):
                self.bootstrap()
                headers["Cookie"] = self._jar.header(GEMINI_COOKIE_HOST)
                form["at"] = self._at
                resp = self._http.post(
                    GEMINI_HOST + "/_/BardChatUi/data/batchexecute",
                    params=params,
                    data=form,
                    headers=headers,
                )
            if resp.status_code != 200:
                raise GeminiWebError(
                    f"batchexecute returned HTTP {resp.status_code}"
                )

    def _parse_conversations(self, raw: str) -> list[dict[str, Any]]:
        """Parse the list of conversations from the MaZiqc response."""
        conversations: list[dict[str, Any]] = []

        for raw_line in raw.splitlines():
            line = raw_line.strip()
            if not line.startswith("[["):
                continue
            try:
                chunk = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(chunk, list):
                continue
            for item in chunk:
                if (
                    isinstance(item, list)
                    and len(item) >= 3
                    and item[0] == "wrb.fr"
                    and item[1] == "MaZiqc"
                    and isinstance(item[2], str)
                    and item[2]
                ):
                    try:
                        body = json.loads(item[2])
                    except json.JSONDecodeError:
                        continue

                    found = self._collect_conversations_from_json(body)
                    for cid, title, ts in found:
                        if not any(c["cid"] == cid for c in conversations):
                            conversations.append(
                                {
                                    "cid": cid,
                                    "title": title,
                                    "updated_at": ts,
                                }
                            )

        return conversations

    def _collect_conversations_from_json(
        self, val: Any
    ) -> list[tuple[str, str, int | float]]:
        """Recursively collect conversation 3-tuples from the JSON structure."""
        res: list[tuple[str, str, int | float]] = []
        if isinstance(val, list):
            if (
                len(val) >= 3
                and isinstance(val[0], str)
                and val[0].startswith("c_")
                and isinstance(val[1], str)
                and isinstance(val[2], (int, float))
            ):
                res.append((val[0], val[1], val[2]))

            for item in val:
                res.extend(self._collect_conversations_from_json(item))
        elif isinstance(val, dict):
            for k, v in val.items():
                res.extend(self._collect_conversations_from_json(v))
        return res

    # -- handshake -----------------------------------------------------------

    def _base_headers(self) -> dict[str, str]:
        return {
            "User-Agent": self._ua,
            "Cookie": self._jar.header(GEMINI_COOKIE_HOST),
        }

    def bootstrap(self) -> str:
        """Fetch ``/app`` and scrape the per-session ``at``/``bl`` tokens.

        Returns:
            The ``SNlM0e`` anti-forgery token (also stored on the instance).

        Raises:
            GeminiWebError: The page could not be loaded or the user is not
                signed in (token absent — usually stale cookies).
        """
        import httpx

        try:
            resp = self._http.get(
                GEMINI_APP_URL,
                headers={**self._base_headers(), "Referer": GEMINI_HOST + "/"},
            )
        except httpx.HTTPError as exc:
            raise GeminiWebError(f"GET {GEMINI_APP_URL} failed: {exc}") from exc

        if resp.status_code != 200:
            raise GeminiWebError(
                f"GET /app returned HTTP {resp.status_code} "
                "(cookies rejected or redirected to sign-in)"
            )

        html = resp.text
        at_match = _SNLM0E_RE.search(html)
        if not at_match:
            raise GeminiWebError(
                "SNlM0e token not found — not signed in. The Google session "
                "cookies are missing or stale; re-import them "
                "(aphrody cookies import <cookie-editor.json>)."
            )
        self._at = at_match.group(1)
        bl_match = _CFB2H_RE.search(html)
        self._bl = bl_match.group(1) if bl_match else DEFAULT_BL
        return self._at

    # -- generation ----------------------------------------------------------

    def generate(
        self,
        prompt: str,
        *,
        keep_context: bool = True,
        model: str | None = None,
    ) -> str:
        """Send *prompt* to the Gemini web app and return the reply text.

        Args:
            prompt: The user message.
            keep_context: When ``True`` (default), thread this turn onto the
                ongoing conversation and remember the new ids.
            model: Override the default model for this request.

        Returns:
            The model's reply text (empty string if the model returned none).

        Raises:
            GeminiWebError: The request failed or the response was unparseable.
        """
        import httpx

        if self._at is None:
            self.bootstrap()

        context = [self._cid, self._rid, self._rcid] if keep_context else None
        inner = json.dumps([[prompt], None, context])
        f_req = json.dumps([None, inner])

        params = {
            "bl": self._bl or DEFAULT_BL,
            "_reqid": str(random.randint(10000, 99999)),
            "rt": "c",
        }
        form = {"at": self._at, "f.req": f_req}
        headers = {
            **self._base_headers(),
            "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
            "Origin": GEMINI_HOST,
            "Referer": GEMINI_APP_URL,
            "X-Same-Domain": "1",
            "x-goog-ext-525001261-jspb": self._get_model_header(
                model or self.model
            ),
        }

        try:
            resp = self._http.post(
                GEMINI_HOST + STREAM_GENERATE_PATH,
                params=params,
                data=form,
                headers=headers,
            )
        except httpx.HTTPError as exc:
            raise GeminiWebError(
                f"StreamGenerate request failed: {exc}"
            ) from exc

        if resp.status_code != 200:
            # A 401/400 here almost always means the at token expired; one
            # re-bootstrap + retry recovers a rotated session cheaply.
            if resp.status_code in (400, 401, 403):
                self.bootstrap()
                headers["Cookie"] = self._jar.header(GEMINI_COOKIE_HOST)
                form["at"] = self._at
                resp = self._http.post(
                    GEMINI_HOST + STREAM_GENERATE_PATH,
                    params=params,
                    data=form,
                    headers=headers,
                )
            if resp.status_code != 200:
                raise GeminiWebError(
                    f"StreamGenerate returned HTTP {resp.status_code}"
                )

        _maybe_dump(resp.text)
        text, (cid, rid, rcid) = self._parse_stream(resp.text)
        if keep_context:
            self._cid, self._rid, self._rcid = cid, rid, rcid
        return text

    # -- response parsing ----------------------------------------------------

    def _parse_stream(
        self, raw: str
    ) -> tuple[str, tuple[str | None, str | None, str | None]]:
        """Decode the chunked StreamGenerate response.

        The body is a sequence of length-prefixed JSON chunks guarded by the
        ``)]}'`` XSSI prefix. The useful chunk is a ``wrb.fr`` envelope whose
        third element is a JSON string carrying the actual reply.

        Args:
            raw: The raw response text.

        Returns:
            ``(reply_text, (cid, rid, rcid))``.

        Raises:
            GeminiWebError: No decodable ``wrb.fr`` payload was found.
        """
        main: Any = None
        for raw_line in raw.splitlines():
            line = raw_line.strip()
            if not line.startswith("[["):
                continue
            try:
                chunk = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(chunk, list):
                continue
            for item in chunk:
                if (
                    isinstance(item, list)
                    and len(item) >= 3
                    and item[0] == "wrb.fr"
                    and isinstance(item[2], str)
                    and item[2]
                ):
                    try:
                        body = json.loads(item[2])
                    except json.JSONDecodeError:
                        continue
                    if isinstance(body, dict):
                        if (
                            "11" in body
                            and isinstance(body["11"], list)
                            and body["11"]
                        ):
                            self.last_title = body["11"][0]
                    elif isinstance(body, list):
                        # Prefer the envelope that actually carries candidates.
                        if len(body) > 4 and body[4]:
                            main = body
                        elif main is None:
                            main = body

        if main is None:
            raise GeminiWebError(
                "no 'wrb.fr' payload with content in StreamGenerate response "
                "(set APHRODY_WEB_DEBUG=1 to dump the raw body)"
            )
        return _extract_reply(main)

    def _get_model_header(self, model_id: str) -> str:
        """Build the x-goog-ext-525001261-jspb header for the chosen model."""
        tokens = {
            "flash-lite": ("1d44b34bcaa1c04d", 6),
            "flashlite": ("1d44b34bcaa1c04d", 6),
            "flash": ("56fdd199312815e2", 1),
            "pro": ("e6fa609c3fa255c0", 3),
        }
        key = model_id.lower().strip()
        if key not in tokens:
            key = "flash"
        token, variant = tokens[key]
        arr = [
            1,
            None,
            None,
            None,
            token,
            None,
            None,
            0,
            [4, 5, 6, 8],
            None,
            None,
            3,
            None,
            None,
            variant,
            1,
            self.client_uuid,
        ]
        return json.dumps(arr, separators=(",", ":"))


def _extract_reply(
    body: list,
) -> tuple[str, tuple[str | None, str | None, str | None]]:
    """Pull reply text and conversation ids from a decoded ``wrb.fr`` body.

    Wire layout (long-standing Bard/Gemini): ``body[1] = [cid, rid]`` and
    ``body[4][0] = [rcid, [reply_text, ...], ...]``.

    Args:
        body: The decoded inner payload (a list).

    Returns:
        ``(reply_text, (cid, rid, rcid))``.
    """
    cid = rid = rcid = None
    if len(body) > 1 and isinstance(body[1], list):
        meta = body[1]
        cid = meta[0] if len(meta) > 0 else None
        rid = meta[1] if len(meta) > 1 else None

    text = ""
    if len(body) > 4 and isinstance(body[4], list) and body[4]:
        candidate = body[4][0]
        if isinstance(candidate, list):
            if candidate and isinstance(candidate[0], str):
                rcid = candidate[0]
            if (
                len(candidate) > 1
                and isinstance(candidate[1], list)
                and candidate[1]
                and isinstance(candidate[1][0], str)
            ):
                text = candidate[1][0]
    return text, (cid, rid, rcid)


def _maybe_dump(raw: str) -> None:
    """Dump the raw response to ``var/forks/gemini_web_raw.txt`` if debugging.

    Enabled by setting ``APHRODY_WEB_DEBUG=1``; aids wire-index debugging.
    """
    if os.environ.get("APHRODY_WEB_DEBUG") != "1":
        return
    try:
        dest = Path("var") / "forks" / "gemini_web_raw.txt"
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(raw, encoding="utf-8")
    except OSError:
        pass


def generate(
    prompt: str,
    *,
    keep_context: bool = False,
    model: str | None = None,
) -> str:
    """One-shot convenience: ask the Gemini web app a single question.

    Args:
        prompt: The user message.
        keep_context: Whether to keep conversation context (irrelevant for a
            one-shot call; defaults to ``False``).
        model: The Gemini model id.

    Returns:
        The model's reply text.
    """
    with GeminiWebClient(model=model or "flash") as client:
        return client.generate(prompt, keep_context=keep_context)
