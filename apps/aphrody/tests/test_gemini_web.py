# SPDX-License-Identifier: Apache-2.0
"""Tests for the Gemini web (Boq) wire parser in :mod:`aphrody.gemini_web`."""

from __future__ import annotations

import json

import httpx
from aphrody.auth.cookies import Cookie, CookieJar
from aphrody.gemini_web import GeminiWebClient, _extract_reply


def test_extract_reply() -> None:
    body = [None, ["cid", "rid"], None, None, [["rcid", ["Hello world"]]]]
    text, ids = _extract_reply(body)
    assert text == "Hello world"
    assert ids == ("cid", "rid", "rcid")


def test_extract_reply_empty() -> None:
    text, ids = _extract_reply([None, None])
    assert text == ""
    assert ids == (None, None, None)


def test_parse_stream() -> None:
    inner = json.dumps([None, ["c", "r"], None, None, [["rc", ["Bonjour"]]]])
    chunk = json.dumps([["wrb.fr", "abc", inner, None, None, None, "generic"]])
    raw = ")]}'\n\n" + str(len(chunk)) + "\n" + chunk + "\n"

    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        text, ids = client._parse_stream(raw)
    assert text == "Bonjour"
    assert ids == ("c", "r", "rc")
