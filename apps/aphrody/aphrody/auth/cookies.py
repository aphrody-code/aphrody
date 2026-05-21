# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Google cookie jar: the keyless credential for the Gemini *web* surface.

The Gemini web app (``gemini.google.com``) is a Boq frontend; it does not use
an OAuth Bearer token but the ordinary Google session cookies
(``__Secure-1PSID``, ``__Secure-1PSIDTS``, ``SAPISID`` ...). This module manages
those cookies for aphrody **without any API key**: it stores them in a private
file, builds the ``Cookie`` header for a given host, and can import them either
from a Cookie-Editor export or — best effort — straight from a local Chrome
profile.

Storage:
    ``~/.aphrody/google-cookies.json`` (override with ``APHRODY_COOKIES_PATH``),
    written ``0o600``. The file holds only the user's own cookies and is never
    committed; values are never logged or printed by :func:`status`.

Note on Chrome App-Bound Encryption (ABE):
    Recent Chrome builds wrap the cookie key with ABE, which defeats library
    decryptors such as ``browser-cookie3``. When :func:`extract_from_chrome`
    fails for that reason, export the cookies once with the Cookie-Editor
    extension and feed them to :func:`import_cookie_editor`.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path

from aphrody.errors import AphrodyError


class CookieStoreError(AphrodyError):
    """The cookie store is missing, empty, or could not be read/extracted."""


#: Cookies that the Boq Gemini web endpoint needs to authenticate a session.
REQUIRED_COOKIES: tuple[str, ...] = ("__Secure-1PSID",)

#: Cookies that materially improve session stability when present.
RECOMMENDED_COOKIES: tuple[str, ...] = (
    "__Secure-1PSID",
    "__Secure-1PSIDTS",
    "__Secure-1PSIDCC",
    "SAPISID",
)


def cookies_path() -> Path:
    """Return the path of the private cookie store.

    Honors ``APHRODY_COOKIES_PATH``; otherwise ``~/.aphrody/google-cookies.json``.

    Returns:
        The resolved store path (the file may not yet exist).
    """
    override = os.environ.get("APHRODY_COOKIES_PATH")
    if override:
        return Path(override).expanduser()
    return Path.home() / ".aphrody" / "google-cookies.json"


def _domain_match(cookie_domain: str, host: str) -> bool:
    """Return whether *cookie_domain* should be sent to *host*.

    Implements the cookie domain-match rule: a leading dot is ignored and the
    cookie applies to the bare domain and all of its subdomains.

    Args:
        cookie_domain: The cookie's ``Domain`` attribute (e.g. ``".google.com"``).
        host: The request host (e.g. ``"gemini.google.com"``).
    """
    d = cookie_domain.lower().lstrip(".")
    h = host.lower()
    return h == d or h.endswith("." + d)


@dataclass(slots=True)
class Cookie:
    """A single HTTP cookie (the subset aphrody needs to send one back)."""

    name: str
    value: str
    domain: str = ".google.com"
    path: str = "/"
    secure: bool = True
    http_only: bool = False
    expiry: float | None = None

    @classmethod
    def from_dict(cls, data: dict) -> Cookie:
        """Build a :class:`Cookie` from a loose dict (Cookie-Editor or native).

        Accepts both aphrody's own keys and the Cookie-Editor export keys
        (``httpOnly``, ``hostOnly``, ``expirationDate``).

        Args:
            data: A mapping with at least ``name`` and ``value``.

        Returns:
            The parsed :class:`Cookie`.
        """
        expiry = data.get("expiry")
        if expiry is None:
            expiry = data.get("expirationDate")
        return cls(
            name=data["name"],
            value=data["value"],
            domain=data.get("domain", ".google.com"),
            path=data.get("path", "/"),
            secure=bool(data.get("secure", True)),
            http_only=bool(data.get("http_only", data.get("httpOnly", False))),
            expiry=float(expiry) if expiry is not None else None,
        )

    def to_dict(self) -> dict:
        """Serialize to aphrody's on-disk cookie shape."""
        out: dict = {
            "name": self.name,
            "value": self.value,
            "domain": self.domain,
            "path": self.path,
            "secure": self.secure,
            "httpOnly": self.http_only,
        }
        if self.expiry is not None:
            out["expiry"] = self.expiry
        return out


@dataclass(slots=True)
class CookieJar:
    """An ordered collection of :class:`Cookie` objects with header helpers."""

    cookies: list[Cookie] = field(default_factory=list)

    # -- introspection -------------------------------------------------------

    def __len__(self) -> int:
        return len(self.cookies)

    def names(self) -> list[str]:
        """All cookie names, in stored order."""
        return [c.name for c in self.cookies]

    def domains(self) -> list[str]:
        """The distinct domains present in the jar."""
        seen: list[str] = []
        for c in self.cookies:
            if c.domain not in seen:
                seen.append(c.domain)
        return seen

    def get(self, name: str) -> Cookie | None:
        """Return the cookie named *name*, or ``None``."""
        for c in self.cookies:
            if c.name == name:
                return c
        return None

    def require(self, *names: str) -> None:
        """Raise :class:`CookieStoreError` if any of *names* is missing.

        Args:
            *names: Cookie names that must be present.

        Raises:
            CookieStoreError: One or more required cookies are absent.
        """
        missing = [
            n for n in (names or REQUIRED_COOKIES) if self.get(n) is None
        ]
        if missing:
            raise CookieStoreError(
                "missing required cookie(s): "
                + ", ".join(missing)
                + f" (have {len(self.cookies)} cookie(s); re-import from Chrome)"
            )

    # -- header building -----------------------------------------------------

    def for_host(self, host: str) -> list[Cookie]:
        """Return the cookies that apply to *host* (domain match)."""
        return [c for c in self.cookies if _domain_match(c.domain, host)]

    def header(self, host: str) -> str:
        """Build the ``Cookie`` request header value for *host*.

        Args:
            host: The target host (e.g. ``"gemini.google.com"``).

        Returns:
            A ``"name=value; name=value"`` string of all applicable cookies.

        Raises:
            CookieStoreError: No stored cookie applies to *host*.
        """
        applicable = self.for_host(host)
        if not applicable:
            raise CookieStoreError(f"no stored cookie applies to host {host!r}")
        return "; ".join(f"{c.name}={c.value}" for c in applicable)

    # -- (de)serialization ---------------------------------------------------

    @classmethod
    def from_list(cls, data: list[dict]) -> CookieJar:
        """Build a jar from a list of cookie dicts."""
        return cls([Cookie.from_dict(d) for d in data])

    def to_list(self) -> list[dict]:
        """Serialize the jar to a list of cookie dicts."""
        return [c.to_dict() for c in self.cookies]


# ---------------------------------------------------------------------------
# Store I/O
# ---------------------------------------------------------------------------


def load(path: Path | None = None) -> CookieJar:
    """Load the cookie jar from the private store.

    Args:
        path: Optional explicit store path; defaults to :func:`cookies_path`.

    Returns:
        The loaded :class:`CookieJar`.

    Raises:
        CookieStoreError: The store does not exist or is malformed/empty.
    """
    p = path or cookies_path()
    if not p.exists():
        raise CookieStoreError(
            f"cookie store not found at {p}; import cookies first "
            "(aphrody cookies import <cookie-editor.json>)"
        )
    try:
        raw = json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CookieStoreError(f"cannot read cookie store {p}: {exc}") from exc

    data = raw.get("cookies", raw) if isinstance(raw, dict) else raw
    if not isinstance(data, list) or not data:
        raise CookieStoreError(f"cookie store {p} is empty")
    return CookieJar.from_list(data)


def save(jar: CookieJar, path: Path | None = None) -> Path:
    """Persist *jar* to the private store with ``0o600`` permissions.

    Args:
        jar: The jar to write.
        path: Optional explicit store path; defaults to :func:`cookies_path`.

    Returns:
        The path written.
    """
    p = path or cookies_path()
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(".tmp")
    tmp.write_text(
        json.dumps(jar.to_list(), indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    try:
        os.chmod(tmp, 0o600)
    except OSError:
        pass
    tmp.replace(p)
    return p


# ---------------------------------------------------------------------------
# Import paths
# ---------------------------------------------------------------------------


def import_cookie_editor(
    data: str | list, *, save_to_store: bool = True
) -> CookieJar:
    """Import cookies from a Cookie-Editor JSON export.

    Args:
        data: Either the export JSON text or an already-parsed list of dicts.
        save_to_store: When ``True``, persist the imported jar to the store.

    Returns:
        The imported :class:`CookieJar`.

    Raises:
        CookieStoreError: The payload is not a non-empty list of cookies.
    """
    if isinstance(data, str):
        try:
            data = json.loads(data)
        except json.JSONDecodeError as exc:
            raise CookieStoreError(
                f"invalid Cookie-Editor JSON: {exc}"
            ) from exc
    if not isinstance(data, list) or not data:
        raise CookieStoreError("Cookie-Editor export must be a non-empty list")

    jar = CookieJar.from_list(data)
    if save_to_store:
        save(jar)
    return jar


def extract_from_chrome(
    domain: str = "google.com", *, save_to_store: bool = True
) -> CookieJar:
    """Best-effort extraction of Google cookies straight from local Chrome.

    Uses the optional ``browser-cookie3`` dependency. On modern Chrome builds
    App-Bound Encryption usually defeats it; the raised error explains the
    Cookie-Editor fallback.

    Args:
        domain: Cookie domain filter (default ``"google.com"``).
        save_to_store: When ``True``, persist a successful extraction.

    Returns:
        The extracted :class:`CookieJar`.

    Raises:
        CookieStoreError: ``browser-cookie3`` is missing or decryption failed.
    """
    try:
        import browser_cookie3  # type: ignore[import-not-found]
    except ImportError as exc:
        raise CookieStoreError(
            "browser-cookie3 is not installed; run "
            "`uv pip install aphrody[cookies]` or import a Cookie-Editor export"
        ) from exc

    try:
        cj = browser_cookie3.chrome(domain_name=domain)
    except Exception as exc:
        raise CookieStoreError(
            "Chrome cookie decryption failed (likely App-Bound Encryption). "
            "Export with the Cookie-Editor extension and run "
            "`aphrody cookies import <file.json>`. Original error: "
            f"{exc}"
        ) from exc

    cookies = [
        Cookie(
            name=c.name,
            value=c.value,
            domain=c.domain,
            path=c.path,
            secure=bool(getattr(c, "secure", True)),
            expiry=float(c.expires) if getattr(c, "expires", None) else None,
        )
        for c in cj
    ]
    jar = CookieJar(cookies)
    if not jar.cookies:
        raise CookieStoreError(f"no cookies extracted for domain {domain!r}")
    if save_to_store:
        save(jar)
    return jar


# ---------------------------------------------------------------------------
# Status (metadata only — never values)
# ---------------------------------------------------------------------------


def status(jar: CookieJar | None = None) -> dict:
    """Return non-sensitive metadata about the stored cookie jar.

    The returned dict **never** contains cookie values — only names, domains,
    counts and which key cookies are present.

    Args:
        jar: An already-loaded jar; loaded from the store when omitted.

    Returns:
        A metadata dict safe to print.
    """
    if jar is None:
        jar = load()
    present = set(jar.names())
    return {
        "path": str(cookies_path()),
        "count": len(jar),
        "domains": jar.domains(),
        "names": jar.names(),
        "has_required": all(n in present for n in REQUIRED_COOKIES),
        "recommended_present": [n for n in RECOMMENDED_COOKIES if n in present],
        "recommended_missing": [
            n for n in RECOMMENDED_COOKIES if n not in present
        ],
    }
