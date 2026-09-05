# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.endpoints`."""

from __future__ import annotations

from aphrody import endpoints


def test_cloud_code_url() -> None:
    ep = endpoints.CloudCodeEndpoint.PROD
    assert ep.host == "https://cloudcode-pa.googleapis.com"
    assert ep.url(endpoints.METHOD_FETCH_AVAILABLE_MODELS) == (
        "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels"
    )


def test_daily_is_default() -> None:
    assert endpoints.DEFAULT_CLOUD_CODE_ENDPOINT is (
        endpoints.CloudCodeEndpoint.DAILY
    )


def test_gemini_generate_content_url() -> None:
    assert endpoints.gemini_generate_content_url("gemini-2.5-flash").endswith(
        "/v1beta/models/gemini-2.5-flash:generateContent"
    )
    assert endpoints.gemini_generate_content_url("m", stream=True).endswith(
        ":streamGenerateContent"
    )


def test_client_id_shape() -> None:
    assert endpoints.ANTIGRAVITY_CLIENT_ID.endswith(
        ".apps.googleusercontent.com"
    )


def test_cloud_platform_scope_present() -> None:
    assert (
        "https://www.googleapis.com/auth/cloud-platform"
        in endpoints.ANTIGRAVITY_SCOPES
    )
