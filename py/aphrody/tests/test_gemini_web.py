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


def test_parse_stream_title() -> None:
    inner_title = json.dumps({"11": ["Test Conversation Title"]})
    inner = json.dumps([None, ["c", "r"], None, None, [["rc", ["Bonjour"]]]])
    chunk_title = json.dumps(
        [["wrb.fr", "abc", inner_title, None, None, None, "generic"]]
    )
    chunk_body = json.dumps(
        [["wrb.fr", "def", inner, None, None, None, "generic"]]
    )
    raw = (
        ")]}'\n\n"
        + str(len(chunk_title))
        + "\n"
        + chunk_title
        + "\n"
        + str(len(chunk_body))
        + "\n"
        + chunk_body
        + "\n"
    )

    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        text, _ = client._parse_stream(raw)
        assert client.last_title == "Test Conversation Title"
    assert text == "Bonjour"


def test_resume() -> None:
    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        client.resume("c1", "r1", "rc1")
        assert client.conversation == ("c1", "r1", "rc1")


def test_model_header() -> None:
    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        # Test default and custom models
        h_flash = client._get_model_header("flash")
        v_flash = json.loads(h_flash)
        assert v_flash[4] == "56fdd199312815e2"
        assert v_flash[14] == 1
        assert v_flash[16] == "00000000-0000-4000-8000-000000000001"

        h_lite = client._get_model_header("flash-lite")
        v_lite = json.loads(h_lite)
        assert v_lite[4] == "1d44b34bcaa1c04d"
        assert v_lite[14] == 6

        h_pro = client._get_model_header("pro")
        v_pro = json.loads(h_pro)
        assert v_pro[4] == "e6fa609c3fa255c0"
        assert v_pro[14] == 3


def test_generate_passes_model_header(httpx_mock) -> None:
    # Setup mock for bootstrap
    httpx_mock.add_response(
        url="https://gemini.google.com/app",
        text='"SNlM0e":"test_at_token","cfb2h":"test_bl_token"',
    )

    # Setup mock for StreamGenerate
    inner = json.dumps([None, ["c", "r"], None, None, [["rc", ["Bonjour"]]]])
    chunk = json.dumps([["wrb.fr", "abc", inner, None, None, None, "generic"]])
    raw = ")]}'\n\n" + str(len(chunk)) + "\n" + chunk + "\n"

    def match_request(request: httpx.Request) -> bool:
        # Check that x-goog-ext-525001261-jspb is in headers
        header_val = request.headers.get("x-goog-ext-525001261-jspb")
        if not header_val:
            return False
        v = json.loads(header_val)
        return v[4] == "e6fa609c3fa255c0"  # Pro model token

    import re

    httpx_mock.add_response(
        url=re.compile(
            r"https://gemini\.google\.com/_/BardChatUi/data/assistant\.lamda\.BardFrontendService/StreamGenerate.*"
        ),
        text=raw,
        match_content=None,
    )

    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client(), model="pro") as client:
        reply = client.generate("hello")
        assert reply == "Bonjour"

    # Verify that the StreamGenerate request had the header
    requests = httpx_mock.get_requests()
    stream_gen_reqs = [r for r in requests if "StreamGenerate" in str(r.url)]
    assert len(stream_gen_reqs) == 1
    req = stream_gen_reqs[0]
    header_val = req.headers.get("x-goog-ext-525001261-jspb")
    assert header_val is not None
    v = json.loads(header_val)
    assert v[4] == "e6fa609c3fa255c0"


def test_collect_conversations_from_json() -> None:
    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        # Nested layout mock
        nested_data = [
            "some_string",
            123,
            [
                ["c_conv1", "Title 1", 1716000000.0],
                ["c_conv2", "Title 2", 1716000001.0],
            ],
            {"some_key": ["c_conv3", "Title 3", 1716000002.0]},
            # Not a conversation because first element doesn't start with c_
            ["not_c_conv", "Bad Title", 1716000003.0],
            # Not a conversation because timestamp is not a number
            ["c_conv_bad_ts", "Bad TS", "not a number"],
        ]
        res = client._collect_conversations_from_json(nested_data)
        assert len(res) == 3
        assert res[0] == ("c_conv1", "Title 1", 1716000000.0)
        assert res[1] == ("c_conv2", "Title 2", 1716000001.0)
        assert res[2] == ("c_conv3", "Title 3", 1716000002.0)


def test_list_conversations(httpx_mock) -> None:
    # Bootstrap mock
    httpx_mock.add_response(
        url="https://gemini.google.com/app",
        text='"SNlM0e":"test_at_token","cfb2h":"test_bl_token"',
    )

    # batchexecute mock
    inner_payload = json.dumps(
        [
            [
                ["c_1", "Conversation One", 1716000000],
                ["c_2", "Conversation Two", 1716000001],
            ]
        ]
    )
    chunk = json.dumps(
        [["wrb.fr", "MaZiqc", inner_payload, None, None, None, "generic"]]
    )
    raw_response = ")]}'\n\n" + str(len(chunk)) + "\n" + chunk + "\n"

    import re

    httpx_mock.add_response(
        url=re.compile(
            r"https://gemini\.google\.com/_/BardChatUi/data/batchexecute.*"
        ),
        text=raw_response,
    )

    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        convs = client.list_conversations()
        assert len(convs) == 2
        assert convs[0] == {
            "cid": "c_1",
            "title": "Conversation One",
            "updated_at": 1716000000,
        }
        assert convs[1] == {
            "cid": "c_2",
            "title": "Conversation Two",
            "updated_at": 1716000001,
        }


def test_delete_conversation(httpx_mock) -> None:
    # Bootstrap mock
    httpx_mock.add_response(
        url="https://gemini.google.com/app",
        text='"SNlM0e":"test_at_token","cfb2h":"test_bl_token"',
    )

    # batchexecute mock for deletion
    chunk = json.dumps(
        [["wrb.fr", "GzXR5e", "null", None, None, None, "generic"]]
    )
    raw_response = ")]}'\n\n" + str(len(chunk)) + "\n" + chunk + "\n"

    import re

    httpx_mock.add_response(
        url=re.compile(
            r"https://gemini\.google\.com/_/BardChatUi/data/batchexecute.*"
        ),
        text=raw_response,
    )

    jar = CookieJar([Cookie("__Secure-1PSID", "x", ".google.com")])
    with GeminiWebClient(jar=jar, http=httpx.Client()) as client:
        client.delete_conversation("c_1")
        # No exception means success
