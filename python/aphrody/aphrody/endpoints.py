# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Public network surface of Google's AI Ultra (Antigravity / Gemini) stack.

These constants were reverse-engineered from the Antigravity 2.0.1 desktop
client and confirmed against its live OAuth flow (2026-05-21). They are **not
secrets**: OAuth client IDs and endpoint hosts are embedded in every
distributed copy of the application, and the desktop flow uses
Authorization-Code + PKCE (no confidential client secret). User tokens are
never stored here; they are read at runtime from the credential store.

This module is the Python mirror of the native Rust crate's
``antigravity_sdk::endpoints`` and ``antigravity_sdk::auth`` constants.
"""

from __future__ import annotations

import enum

# ---------------------------------------------------------------------------
# OAuth 2.0
# ---------------------------------------------------------------------------

#: Primary OAuth 2.0 client id used by the Antigravity CLI (``agy``).
#: Confirmed as the ``aud`` of the live ``gemini:antigravity`` token.
ANTIGRAVITY_CLIENT_ID = (
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
)

#: Secondary OAuth 2.0 client id also embedded in the desktop binary.
ANTIGRAVITY_CLIENT_ID_SECONDARY = (
    "884354919052-36trc1jjb3tguiac32ov6cod268c5blh.apps.googleusercontent.com"
)

#: Loopback redirect URI used by the desktop sign-in flow.
OAUTH_REDIRECT_URI = "http://localhost:9109/oauth-callback"

#: Google OAuth 2.0 authorization endpoint (browser redirect target).
OAUTH_AUTHORIZATION_ENDPOINT = "https://accounts.google.com/o/oauth2/v2/auth"

#: Google OAuth 2.0 token endpoint (code exchange + refresh).
OAUTH_TOKEN_ENDPOINT = "https://oauth2.googleapis.com/token"

#: Google OAuth 2.0 ``tokeninfo`` endpoint (validate an access token).
OAUTH_TOKENINFO_ENDPOINT = "https://www.googleapis.com/oauth2/v3/tokeninfo"

#: Google OpenID ``userinfo`` endpoint (email + profile of the signed-in user).
OAUTH_USERINFO_ENDPOINT = "https://www.googleapis.com/oauth2/v2/userinfo"

#: Scopes that the Antigravity desktop client requests in the live sign-in
#: flow (verified against the running 2.0.1 client, 2026-05-21).
ANTIGRAVITY_SCOPES = (
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/cclog",
    "https://www.googleapis.com/auth/experimentsandconfigs",
)

# ---------------------------------------------------------------------------
# API hosts
# ---------------------------------------------------------------------------

#: Gemini generative-language API host (``generateContent``, models, etc.).
GEMINI_API_HOST = "https://generativelanguage.googleapis.com"

#: Vertex AI host (publishers/google + publishers/anthropic models).
VERTEX_AI_HOST = "https://aiplatform.googleapis.com"


class CloudCodeEndpoint(enum.Enum):
    """Cloud Code "Private API" host.

    The desktop client defaults its runtime to :attr:`DAILY` but ships
    :attr:`PROD` as the stable surface.
    """

    PROD = "https://cloudcode-pa.googleapis.com"
    DAILY = "https://daily-cloudcode-pa.googleapis.com"

    @property
    def host(self) -> str:
        """The HTTPS host (no trailing slash) for this endpoint."""
        return self.value

    def url(self, method: str) -> str:
        """Build the absolute URL for a Cloud Code ``method`` on this host.

        Args:
            method: One of the ``METHOD_*`` path constants in this module
                (for example :data:`METHOD_FETCH_AVAILABLE_MODELS`).

        Returns:
            The absolute ``https://`` URL for the method.
        """
        return f"{self.host}{method}"


#: Default Cloud Code endpoint, mirroring the desktop client runtime default.
DEFAULT_CLOUD_CODE_ENDPOINT = CloudCodeEndpoint.DAILY

# ---------------------------------------------------------------------------
# Cloud Code ``v1internal`` method paths
# ---------------------------------------------------------------------------

#: ``POST /v1internal:loadCodeAssist`` — bootstrap the Code Assist session
#: (project, tier, entitlements) for the signed-in user.
METHOD_LOAD_CODE_ASSIST = "/v1internal:loadCodeAssist"

#: ``POST /v1internal:fetchAvailableModels`` — list models available to the
#: user's account / tier.
METHOD_FETCH_AVAILABLE_MODELS = "/v1internal:fetchAvailableModels"

#: ``POST /v1internal:onboardUser`` — onboarding / first-run provisioning.
METHOD_ONBOARD_USER = "/v1internal:onboardUser"


def gemini_generate_content_url(model: str, *, stream: bool = False) -> str:
    """Return the Gemini ``generateContent`` URL for ``model``.

    Args:
        model: A bare model id such as ``"gemini-2.5-flash"``.
        stream: When ``True``, target ``streamGenerateContent`` instead.

    Returns:
        The absolute Gemini API URL.
    """
    verb = "streamGenerateContent" if stream else "generateContent"
    return f"{GEMINI_API_HOST}/v1beta/models/{model}:{verb}"
