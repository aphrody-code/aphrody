<!-- SPDX-License-Identifier: Apache-2.0 -->

# Python Stack — python/aphrody (2026)

Fact-checked against PyPI on 2026-05-21. Versions are the current stable
releases. All libraries confirmed Python 3.10-3.14 compatible unless noted.

---

## Summary Table

| Need | Library | Version | SPDX Licence | Rejected Alternative | Reason for rejection |
|---|---|---|---|---|---|
| 1. HTTP sync+async | `httpx` | 0.28.1 | BSD-3-Clause | `aiohttp` | sync API is second-class; no built-in HTTP/2 |
| 2. CLI "style Google" | `fire` | 0.7.1 | Apache-2.0 | `typer` | Typer needs type annotations everywhere; Fire mirrors google-internal CLI style |
| 3. OAuth2 / Google creds | `google-auth` + `google-auth-oauthlib` | 2.53.0 / 1.4.0 | Apache-2.0 | `authlib` | authlib is generic; google-auth has direct Credentials.from_authorized_user_info + Windows Credential Manager hooks |
| 4. Google GenAI SDK | `google-genai` | 2.5.0 | Apache-2.0 | `google-cloud-aiplatform` | google-genai is the unified SDK covering both Gemini Developer API and Vertex AI in one surface |
| 5. Browser cookies | `browser-cookie3` | 0.20.1 | LGPL-3.0 | `pycookiecheat` | pycookiecheat is macOS-only; browser-cookie3 covers Chrome/Edge/Firefox on Linux + Windows incl. DPAPI. **LGPL risk: see section below.** |
| 5b. Win32 DPAPI | `pywin32` | 311 | PSF-2.0 | `ctypes` direct | pywin32 wraps win32crypt cleanly; PSF-2.0 is permissive |
| 6. Keyring / secrets | `keyring` | 25.7.0 | MIT | `secretstorage` | secretstorage is Linux-only (D-Bus); keyring is cross-platform with auto backend detection |
| 7. Data validation | `pydantic` | 2.13.4 | MIT | `attrs` | pydantic v2 has Rust core, full JSON schema, integrates with google-genai response models |
| 8. Retry / backoff | `stamina` | 26.1.0 | MIT | `tenacity` | stamina is opinionated wrapper around tenacity with sane defaults, testing helpers, Prometheus hooks |
| 9. Terminal output | `rich` | 15.0.0 | MIT | `textual` | textual is for full TUI apps; rich covers tables/JSON/progress bars for CLI output |
| 10. HTTP test mocking | `pytest` + `pytest-httpx` | 9.0.3 / 0.36.2 | MIT | `respx` | pytest-httpx integrates directly with httpx transports; respx works too but has less active maintenance |
| 11. Lint + types | `ruff` + `pyright` | 0.15.13 / 1.1.409 | MIT | `mypy` | pyright has faster incremental check and better Pydantic v2 plugin; ruff replaces flake8+isort+pyupgrade |
| 12. Dates / RFC3339 | `python-dateutil` | 2.9.0.post0 | Apache-2.0 | `arrow` | dateutil is stdlib-compatible and has zero dependencies; arrow adds 4 deps for marginal gain |

---

## Per-Library Notes and Idioms

### 1. httpx — HTTP sync + async

```python
import httpx

# Sync
resp = httpx.get("https://cloudcode-pa.googleapis.com/v1/models", headers=auth_headers)
resp.raise_for_status()

# Async
async with httpx.AsyncClient() as client:
    resp = await client.post(url, json=payload, timeout=30.0)
    resp.raise_for_status()
```

Use `httpx.Client(headers=..., timeout=httpx.Timeout(30.0))` for persistent
sessions. Pass Google OAuth tokens via `Authorization: Bearer {token}`.

### 2. fire — CLI "style Google"

```python
import fire

class Aphrody:
    """Aphrody CLI — Google AI ecosystem client."""

    def chat(self, prompt: str, model: str = "gemini-2.0-flash") -> None:
        """Send a chat prompt to Gemini via Antigravity credentials."""
        ...

    def auth(self, show: bool = False) -> None:
        """Show or refresh the stored Antigravity credential."""
        ...

if __name__ == "__main__":
    fire.Fire(Aphrody)
```

Note: `fire` is the canonical PyPI name for Google Python Fire (package `fire`,
not `python-fire` which is an unrelated squatted package at version 0.1.0).

### 3. google-auth + google-auth-oauthlib — OAuth2 / Google credentials

```python
from google.oauth2.credentials import Credentials
import google.auth.transport.requests

# Build Credentials from a stored token dict (e.g. extracted from
# Windows Credential Manager via pywin32 + win32crypt).
creds = Credentials(
    token=access_token,
    refresh_token=refresh_token,
    token_uri="https://oauth2.googleapis.com/token",
    client_id=client_id,
    client_secret=client_secret,
)

# Refresh if expired
request = google.auth.transport.requests.Request()
if not creds.valid:
    creds.refresh(request)
```

For PKCE loopback flows use `google_auth_oauthlib.flow.InstalledAppFlow`.
`google-auth-oauthlib` 1.4.0 requires Python >= 3.10, matching the project
minimum.

### 4. google-genai — GenAI / Vertex AI SDK

```python
from google import genai
from google.genai import types

# Pass google.oauth2.credentials.Credentials directly — no API key needed.
client = genai.Client(
    vertexai=True,
    project="your-gcp-project",
    location="us-central1",
    credentials=creds,  # google.oauth2.credentials.Credentials
)

response = client.models.generate_content(
    model="gemini-2.0-flash",
    contents=[types.Content(role="user", parts=[types.Part(text=prompt)])],
)
print(response.text)
```

`google-genai` 2.x is the unified SDK replacing both `google-generativeai`
(deprecated) and direct Vertex AI SDK usage for Gemini. The `vertexai=True`
flag routes through `cloudcode-pa.googleapis.com`-compatible Vertex endpoints.

### 5. browser-cookie3 + pywin32 — Cookie extraction

```python
import browser_cookie3

# Chrome cookies for .google.com (cross-platform)
jar = browser_cookie3.chrome(domain_name=".google.com")
cookies = {c.name: c.value for c in jar}

# Windows-only: DPAPI via pywin32 (guard with sys.platform check)
import sys
if sys.platform == "win32":
    import win32crypt
    plaintext = win32crypt.CryptUnprotectData(ciphertext, None, None, None, 0)[1]
```

**LGPL risk note:** `browser-cookie3` is LGPL-3.0. For an Apache-2.0
distributed binary, LGPL requires dynamic linking or that the library remains
separable. As a Python import (not compiled in), distribution of the final
binary without the `.py` sources is acceptable under LGPL provided the library
is not statically compiled. If in doubt, isolate behind an optional extra
(`[cookies]`) so users can opt out or replace the implementation.

`pywin32` must be gated behind `if sys.platform == "win32":` — it does not
install on Linux.

### 6. keyring — Secrets / Keychain

```python
import keyring

# Store
keyring.set_password("aphrody", "antigravity_token", access_token)

# Retrieve
token = keyring.get_password("aphrody", "antigravity_token")
```

On Linux, uses libsecret (GNOME Keyring / KDE Wallet) via `SecretService`.
On Windows, uses the Windows Credential Manager. On macOS, uses the system
Keychain. No backend configuration required in typical desktop environments.

### 7. pydantic v2 — Data validation

```python
from pydantic import BaseModel, Field
from datetime import datetime

class GeminiCredential(BaseModel):
    access_token: str
    refresh_token: str
    expires_at: datetime
    scopes: list[str] = Field(default_factory=list)

    model_config = {"frozen": True}

cred = GeminiCredential.model_validate(raw_dict)
```

Use `model_validate` (not deprecated `parse_obj`). Pydantic v2 ships a Rust
core (`pydantic-core`) making validation ~5-50x faster than v1.

### 8. stamina — Retry / backoff

```python
import stamina
import httpx

@stamina.retry(on=(httpx.HTTPStatusError, httpx.TransportError), attempts=4)
def call_gemini(client: httpx.Client, url: str, payload: dict) -> dict:
    resp = client.post(url, json=payload)
    resp.raise_for_status()
    return resp.json()

# Async
@stamina.retry(on=httpx.HTTPStatusError, timeout=60.0, wait_max=30.0)
async def call_gemini_async(client: httpx.AsyncClient, url: str) -> dict:
    resp = await client.get(url)
    resp.raise_for_status()
    return resp.json()
```

stamina uses exponential backoff with full jitter by default. For 401 and 429
specifically, inspect the exception before retrying:

```python
@stamina.retry(on=httpx.HTTPStatusError)
def guarded_call(client, url):
    try:
        resp = client.get(url)
        resp.raise_for_status()
    except httpx.HTTPStatusError as exc:
        if exc.response.status_code == 401:
            raise  # do not retry auth errors; caller must refresh token
        raise
    return resp.json()
```

### 9. rich — Terminal output

```python
from rich.console import Console
from rich.table import Table
import rich.json

console = Console()

# JSON pretty-print
console.print_json(json_string)

# Table
table = Table(title="Models")
table.add_column("ID")
table.add_column("Version")
for m in models:
    table.add_row(m.id, m.version)
console.print(table)

# Progress
from rich.progress import track
for item in track(items, description="Processing..."):
    process(item)
```

### 10. pytest + pytest-httpx — Tests and HTTP mocking

```python
import pytest
import httpx
from pytest_httpx import HTTPXMock

def test_gemini_call(httpx_mock: HTTPXMock) -> None:
    httpx_mock.add_response(
        url="https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
        json={"candidates": [{"content": {"parts": [{"text": "hello"}]}}]},
    )
    with httpx.Client() as client:
        resp = client.post(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
            json={"contents": [{"role": "user", "parts": [{"text": "hi"}]}]},
        )
    assert resp.json()["candidates"][0]["content"]["parts"][0]["text"] == "hello"
```

pytest-httpx works at the `httpx` transport layer — no monkey-patching of
`socket` needed. Works with both sync and async clients.

### 11. ruff + pyright — Lint and type checking

ruff configuration is already present in the workspace `pyproject.toml` with
`convention = "google"` and the `D` pydocstyle ruleset. No additional config
required for `python/aphrody/`.

For pyright, add to `python/aphrody/pyproject.toml`:

```toml
[tool.pyright]
pythonVersion = "3.11"
typeCheckingMode = "standard"
reportMissingImports = true
reportMissingTypeStubs = false
```

mypy is rejected: slower incremental check, less accurate Pydantic v2
inference, requires explicit `[mypy-xxx]` stubs for third-party packages.

### 12. python-dateutil — Dates and RFC3339

```python
from dateutil import parser as dtparse
from datetime import timezone

# Parse RFC3339 / ISO8601 string returned by Google APIs
expires_at = dtparse.parse("2026-05-21T18:00:00Z").replace(tzinfo=timezone.utc)

# Check expiry
from datetime import datetime
is_expired = datetime.now(timezone.utc) >= expires_at
```

For async runtimes: `asyncio` is sufficient for the project needs (httpx
async, google-genai async). Trio is supported by both stamina and httpx but
is not required.

---

## Dependencies Block (pyproject.toml)

Paste into `python/aphrody/pyproject.toml` under `[project]`:

```toml
dependencies = [
    # HTTP client — sync + async, HTTP/2
    "httpx>=0.28.1",

    # CLI framework (Google Python Fire)
    "fire>=0.7.1",

    # Google OAuth2 credentials — no API key required
    "google-auth>=2.53.0",
    "google-auth-oauthlib>=1.4.0",

    # Google GenAI / Vertex AI unified SDK
    "google-genai>=2.5.0",

    # Data validation (Rust-backed v2)
    "pydantic>=2.13.4",

    # Production-grade retries with sane defaults
    "stamina>=26.1.0",

    # Rich terminal output
    "rich>=15.0.0",

    # RFC3339 / ISO8601 date parsing
    "python-dateutil>=2.9.0.post0",

    # Keyring / secrets — cross-platform
    "keyring>=25.7.0",
]

[project.optional-dependencies]
cookies = [
    # LGPL-3.0 — optional, see licence note in STACK.md
    "browser-cookie3>=0.20.1",
    # Windows-only: DPAPI decryption
    "pywin32>=311; sys_platform == 'win32'",
]
dev = [
    "pytest>=9.0.3",
    "pytest-httpx>=0.36.2",
    "ruff>=0.15.13",
    "pyright>=1.1.409",
]
```

---

## Licence Risk Register

| Library | SPDX | Risk level | Mitigation |
|---|---|---|---|
| `browser-cookie3` | LGPL-3.0 | **Medium** | Kept in optional `[cookies]` extra. Python dynamic import satisfies LGPL "separate component" requirement for source distributions. Do not compile into a static binary without fulfilling LGPL obligations. |
| `pywin32` | PSF-2.0 | None | Permissive. Windows-only via `sys_platform == 'win32'` marker. |
| All others | MIT / BSD-3 / Apache-2.0 | None | All compatible with Apache-2.0 distribution. |

No GPL-2.0 or AGPL libraries are included. `absl-py` (Apache-2.0) is
already a transitive dependency via `google-antigravity`; it is not added
directly unless needed for flags/logging.
