# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Authenticated HTTP client for Google's Cloud Code / Gemini surface.

:class:`AphrodyClient` wraps an ``httpx.Client`` together with an
:class:`~aphrody.auth.tokens.OAuthToken` and injects a ``Bearer`` header on
every request. When a request fails with ``401``/``403`` the client refreshes
the token once (through the proven OAuth path) and retries — so callers never
deal with token expiry.

This is the Python mirror of the native Rust ``antigravity_sdk::client``
``AntigravityClient`` and exposes the same Cloud Code ``v1internal`` methods.
For text generation the higher-level :mod:`aphrody.vertex` module (google-genai
over Vertex AI) is the recommended, proven path; this client also offers a raw
``generate_content`` against the Gemini API for completeness.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    import httpx

from aphrody import endpoints
from aphrody.auth import credential_store, credentials, oauth
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import ApiError, NoRefreshTokenError, OAuthServerError

_REFRESHABLE_STATUSES = frozenset({401, 403})


class AphrodyClient:
    """Authenticated client for the Cloud Code / Gemini API surface.

    Construct with :meth:`from_credential_manager` to load the token
    automatically, or pass an explicit ``token``.

    Every request carries ``Authorization: Bearer <access_token>``; on a
    ``401``/``403`` the token is refreshed once and the request is retried.
    """

    def __init__(
        self,
        *,
        token: OAuthToken | None = None,
        http: httpx.Client | None = None,
        cloud_code_endpoint: endpoints.CloudCodeEndpoint = (
            endpoints.DEFAULT_CLOUD_CODE_ENDPOINT
        ),
        persist_refresh: bool = True,
    ) -> None:
        if http is None:
            import httpx

            self._http = httpx.Client(timeout=60.0)
        else:
            self._http = http
        self._owns_http = http is None
        self._token = token or credentials.load_token(http=self._http)
        self._endpoint = cloud_code_endpoint
        self._persist_refresh = persist_refresh

    @classmethod
    def from_credential_manager(
        cls,
        *,
        cloud_code_endpoint: endpoints.CloudCodeEndpoint = (
            endpoints.DEFAULT_CLOUD_CODE_ENDPOINT
        ),
    ) -> AphrodyClient:
        """Create a client by loading the token from the credential store.

        Args:
            cloud_code_endpoint: Which Cloud Code host to target.

        Returns:
            A ready-to-use :class:`AphrodyClient`.
        """
        return cls(cloud_code_endpoint=cloud_code_endpoint)

    # -- lifecycle -----------------------------------------------------------

    def close(self) -> None:
        """Close the underlying HTTP client if this instance owns it."""
        if self._owns_http:
            self._http.close()

    def __enter__(self) -> AphrodyClient:
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    @property
    def token(self) -> OAuthToken:
        """The currently loaded token."""
        return self._token

    # -- low-level transport -------------------------------------------------

    def _refresh(self) -> None:
        """Obtain a fresher token in place, keylessly.

        Prefers the token the Antigravity app keeps current in the OS
        credential store; only falls back to an OAuth ``refresh_token`` grant
        (which the confidential desktop client would need a secret for) as a
        best effort, swallowing the expected rejection.
        """
        previous = self._token.access_token
        fresh = credential_store.read_token()
        if fresh.access_token != previous:
            self._token = fresh
            return
        try:
            self._token = oauth.refresh(self._token, http=self._http)
        except (OAuthServerError, NoRefreshTokenError):
            self._token = fresh
            return
        if self._persist_refresh:
            try:
                credential_store.write_cache(self._token)
            except OSError:
                pass

    def _send(
        self,
        method: str,
        url: str,
        *,
        json_body: Any | None = None,
        params: dict[str, str] | None = None,
        headers: dict[str, str] | None = None,
    ) -> httpx.Response:
        """Send one request with the current Bearer token."""
        merged = {"Authorization": f"Bearer {self._token.access_token}"}
        if headers:
            merged.update(headers)
        return self._http.request(
            method, url, json=json_body, params=params, headers=merged
        )

    def request_json(
        self,
        method: str,
        url: str,
        *,
        json_body: Any | None = None,
        params: dict[str, str] | None = None,
        headers: dict[str, str] | None = None,
    ) -> Any:
        """Send an authenticated request and return the decoded JSON body.

        On a ``401``/``403`` the token is refreshed once and the request is
        retried before giving up.

        Args:
            method: HTTP method (``"GET"``, ``"POST"``, ...).
            url: Absolute request URL.
            json_body: Optional JSON-serializable request body.
            params: Optional query parameters.
            headers: Optional extra headers.

        Returns:
            The decoded JSON response.

        Raises:
            ApiError: The server returned a non-2xx response (after one
                refresh attempt for auth failures).
        """
        response = self._send(
            method, url, json_body=json_body, params=params, headers=headers
        )
        if (
            response.status_code in _REFRESHABLE_STATUSES
            and self._token.refresh_token
        ):
            self._refresh()
            response = self._send(
                method,
                url,
                json_body=json_body,
                params=params,
                headers=headers,
            )

        if response.status_code // 100 != 2:
            raise ApiError(response.status_code, response.text, url)
        if not response.content:
            return {}
        return response.json()

    def get_json(self, url: str, **kwargs: Any) -> Any:
        """Authenticated GET returning decoded JSON."""
        return self.request_json("GET", url, **kwargs)

    def post_json(self, url: str, body: Any, **kwargs: Any) -> Any:
        """Authenticated POST of ``body`` returning decoded JSON."""
        return self.request_json("POST", url, json_body=body, **kwargs)

    # -- typed surface -------------------------------------------------------

    def userinfo(self) -> dict:
        """Return the signed-in user's OpenID profile (email + name)."""
        return self.get_json(endpoints.OAUTH_USERINFO_ENDPOINT)

    def cloud_code(self, method: str, body: Any | None = None) -> Any:
        """Call a Cloud Code ``v1internal`` method on the configured endpoint.

        Args:
            method: A ``METHOD_*`` path constant from :mod:`aphrody.endpoints`.
            body: The JSON request body (defaults to ``{}``).

        Returns:
            The decoded JSON response.
        """
        return self.post_json(self._endpoint.url(method), body or {})

    def load_code_assist(self, body: Any | None = None) -> Any:
        """``loadCodeAssist`` — bootstrap the Code Assist session/entitlements.

        Args:
            body: Optional request body (project / metadata).

        Returns:
            The decoded JSON response (tier, project, entitlements).
        """
        return self.cloud_code(endpoints.METHOD_LOAD_CODE_ASSIST, body)

    def fetch_available_models(self, body: Any | None = None) -> Any:
        """``fetchAvailableModels`` — list models available to the user."""
        return self.cloud_code(endpoints.METHOD_FETCH_AVAILABLE_MODELS, body)

    def onboard_user(self, body: Any | None = None) -> Any:
        """``onboardUser`` — first-run provisioning for the signed-in user."""
        return self.cloud_code(endpoints.METHOD_ONBOARD_USER, body)

    def generate_content(
        self,
        model: str,
        request: dict,
        *,
        user_project: str | None = None,
    ) -> dict:
        """Raw Gemini ``generateContent`` against the generative-language API.

        This uses the OAuth Bearer token (no API key). When the account needs a
        billing/quota project for the generative-language surface, pass
        ``user_project`` to set the ``x-goog-user-project`` header.

        Args:
            model: A bare model id, e.g. ``"gemini-2.5-flash"``.
            request: The ``generateContent`` request body (``contents``, ...).
            user_project: Optional GCP project for the quota header.

        Returns:
            The decoded ``GenerateContentResponse``.
        """
        headers = (
            {"x-goog-user-project": user_project} if user_project else None
        )
        return self.post_json(
            endpoints.gemini_generate_content_url(model),
            request,
            headers=headers,
        )
