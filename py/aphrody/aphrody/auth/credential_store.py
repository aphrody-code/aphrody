# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Read and persist the Antigravity OAuth token across platforms.

On Windows the source of truth is the **Windows Credential Manager** generic
credential ``gemini:antigravity`` written by the Antigravity CLI. On other
platforms (Linux is cible #1) — and as a refresh cache / fallback everywhere —
aphrody uses a private file ``antigravity-token.json`` inside its secrets
directory (``<repo>/var/secrets`` in-repo, else ``~/.aphrody``), with
owner-only permissions.

No secret is ever logged. The blob is read, parsed, and — for the Win32 path —
copied out of the LSASS allocation before it is freed.
"""

from __future__ import annotations

import json
import os
import stat
import sys
from pathlib import Path

from aphrody import _paths
from aphrody.auth.tokens import OAuthToken
from aphrody.errors import (
    CredentialManagerError,
    EmptyCredentialError,
    TokenParseError,
    UnsupportedPlatformError,
)

#: Credential Manager target name used by the Antigravity CLI.
CRED_TARGET = "gemini:antigravity"

_IS_WINDOWS = sys.platform.startswith("win")


def cache_path() -> Path:
    """Return the path to aphrody's private token file.

    Honors ``APHRODY_TOKEN_PATH``; otherwise the file ``antigravity-token.json``
    inside :func:`aphrody._paths.secrets_dir` (``var/secrets`` in-repo). On
    Linux — where there is no Credential Manager — this file is the token
    source, not just a cache.
    """
    override = os.environ.get("APHRODY_TOKEN_PATH")
    if override:
        return Path(override).expanduser()
    return _paths.secret_file("antigravity-token.json")


# ---------------------------------------------------------------------------
# Windows Credential Manager (ctypes)
# ---------------------------------------------------------------------------

if _IS_WINDOWS:  # pragma: no cover - platform specific
    import ctypes
    from ctypes import wintypes

    _CRED_TYPE_GENERIC = 1
    _CRED_PERSIST_LOCAL_MACHINE = 2

    class _CREDENTIALW(ctypes.Structure):
        _fields_ = [
            ("Flags", wintypes.DWORD),
            ("Type", wintypes.DWORD),
            ("TargetName", wintypes.LPWSTR),
            ("Comment", wintypes.LPWSTR),
            ("LastWritten", wintypes.FILETIME),
            ("CredentialBlobSize", wintypes.DWORD),
            ("CredentialBlob", ctypes.POINTER(ctypes.c_ubyte)),
            ("Persist", wintypes.DWORD),
            ("AttributeCount", wintypes.DWORD),
            ("Attributes", ctypes.c_void_p),
            ("TargetAlias", wintypes.LPWSTR),
            ("UserName", wintypes.LPWSTR),
        ]

    def _advapi32() -> ctypes.WinDLL:
        return ctypes.WinDLL("advapi32", use_last_error=True)

    def read_windows_credential(target: str = CRED_TARGET) -> bytes:
        """Read a generic credential blob from the Windows Credential Manager.

        Args:
            target: The credential target name.

        Returns:
            The raw credential blob bytes.

        Raises:
            CredentialManagerError: ``CredReadW`` failed (e.g. not found).
            EmptyCredentialError: The credential exists but its blob is empty.
        """
        advapi32 = _advapi32()
        cred_read = advapi32.CredReadW
        cred_read.argtypes = [
            wintypes.LPWSTR,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.POINTER(_CREDENTIALW)),
        ]
        cred_read.restype = wintypes.BOOL
        cred_free = advapi32.CredFree
        cred_free.argtypes = [ctypes.c_void_p]
        cred_free.restype = None

        pcred = ctypes.POINTER(_CREDENTIALW)()
        ok = cred_read(target, _CRED_TYPE_GENERIC, 0, ctypes.byref(pcred))
        if not ok or not pcred:
            raise CredentialManagerError(ctypes.get_last_error())
        try:
            cred = pcred.contents
            size = cred.CredentialBlobSize
            if not cred.CredentialBlob or size == 0:
                raise EmptyCredentialError(
                    f"credential {target!r} has an empty blob"
                )
            return bytes(cred.CredentialBlob[:size])
        finally:
            cred_free(pcred)

    def write_windows_credential(
        blob: bytes,
        target: str = CRED_TARGET,
        username: str = "antigravity",
    ) -> None:
        """Write/overwrite a generic credential in the Credential Manager.

        Used to keep the shared ``gemini:antigravity`` entry in sync after a
        refresh. Opt-in: aphrody does not call this unless explicitly asked.

        Args:
            blob: The raw credential blob to store.
            target: The credential target name.
            username: The username field to record.

        Raises:
            CredentialManagerError: ``CredWriteW`` failed.
        """
        advapi32 = _advapi32()
        cred_write = advapi32.CredWriteW
        cred_write.argtypes = [ctypes.POINTER(_CREDENTIALW), wintypes.DWORD]
        cred_write.restype = wintypes.BOOL

        buf = (ctypes.c_ubyte * len(blob)).from_buffer_copy(blob)
        cred = _CREDENTIALW()
        cred.Type = _CRED_TYPE_GENERIC
        cred.TargetName = target
        cred.CredentialBlobSize = len(blob)
        cred.CredentialBlob = ctypes.cast(buf, ctypes.POINTER(ctypes.c_ubyte))
        cred.Persist = _CRED_PERSIST_LOCAL_MACHINE
        cred.UserName = username
        if not cred_write(ctypes.byref(cred), 0):
            raise CredentialManagerError(ctypes.get_last_error())


# ---------------------------------------------------------------------------
# Cross-platform cache file
# ---------------------------------------------------------------------------


def read_cache() -> OAuthToken | None:
    """Read the refreshed-token cache, if present.

    Returns:
        The cached :class:`OAuthToken`, or ``None`` if the cache is absent or
        unreadable.
    """
    path = cache_path()
    if not path.exists():
        return None
    try:
        return OAuthToken.from_blob(path.read_bytes())
    except (json.JSONDecodeError, KeyError, OSError):
        return None


def enforce_private_permissions(path: Path) -> None:
    """Enforce owner-only permissions on a file or directory cross-platform.

    On POSIX systems, applies ``0600`` (owner-only read-write) for files and
    ``0700`` (owner-only read-write-execute) for directories.
    On Windows, uses ``icacls`` to disable inheritance and grant full access
    only to the current user.
    """
    if os.name == "nt":
        username = os.environ.get("USERNAME")
        if username:
            import subprocess

            subprocess.run(
                [
                    "icacls",
                    str(path),
                    "/inheritance:r",
                    "/grant:r",
                    f"{username}:(F)",
                ],
                capture_output=True,
                text=True,
                check=False,
            )
    else:
        try:
            if path.is_dir():
                os.chmod(path, stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
            else:
                os.chmod(path, stat.S_IRUSR | stat.S_IWUSR)
        except OSError:
            pass


def write_cache(token: OAuthToken) -> None:
    """Persist a token to the private cache with owner-only permissions.

    Args:
        token: The token to cache.
    """
    path = cache_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        enforce_private_permissions(path.parent)
    except Exception:
        pass

    tmp = path.with_suffix(".tmp")

    # Secure file creation with O_CREAT and 0o600 permissions
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    if hasattr(os, "O_BINARY"):
        flags |= os.O_BINARY

    blob = token.to_blob()
    try:
        fd = os.open(tmp, flags, 0o600)
        try:
            with open(fd, "wb") as f:
                f.write(blob)
        except Exception:
            os.close(fd)
            raise
    except OSError:
        # Fallback to standard write if os.open is not supported or fails
        tmp.write_bytes(blob)

    try:
        enforce_private_permissions(tmp)
    except Exception:
        pass

    tmp.replace(path)

    try:
        enforce_private_permissions(path)
    except Exception:
        pass


# ---------------------------------------------------------------------------
# Unified read
# ---------------------------------------------------------------------------


def read_token() -> OAuthToken:
    """Read the Antigravity token from the platform's primary source.

    On Windows this is the Credential Manager entry ``gemini:antigravity``; on
    other platforms it is the aphrody cache file.

    Returns:
        The parsed :class:`OAuthToken`.

    Raises:
        CredentialManagerError: The Windows credential was not found.
        EmptyCredentialError: The credential blob was empty.
        TokenParseError: The blob could not be parsed as a token.
        UnsupportedPlatformError: No credential source exists on this platform.
    """
    if _IS_WINDOWS:
        try:
            blob = read_windows_credential()
        except (CredentialManagerError, EmptyCredentialError):
            # No live Credential Manager entry: fall back to the secrets file
            # (e.g. a token dropped into var/secrets/antigravity-token.json).
            cached = read_cache()
            if cached is not None:
                return cached
            raise
        try:
            return OAuthToken.from_blob(blob)
        except (json.JSONDecodeError, KeyError) as exc:
            raise TokenParseError(
                "credential blob is not a valid token envelope"
            ) from exc

    cached = read_cache()
    if cached is None:
        raise UnsupportedPlatformError(
            "no Antigravity token found: expected the Windows Credential "
            f"Manager entry {CRED_TARGET!r} (Windows) or a cache at "
            f"{cache_path()} (other platforms). Run 'aphrody auth login' or "
            "sign in with the Antigravity client first."
        )
    return cached
