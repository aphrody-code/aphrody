# Copyright 2026 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Deep dive example analyzing WebView2 user profile data.

Uses Magika to classify file types, queries SQLite history, and extracts
structured information via LangExtract.
"""

from __future__ import annotations

import ctypes
import json
import os
import shutil
import sqlite3
import tempfile
from ctypes import wintypes
from pathlib import Path
from typing import Any

import langextract as lx
from magika import Magika


class CREDENTIALW(ctypes.Structure):
    """Structure representing a Windows credential."""

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


def get_gemini_credentials() -> tuple[Any | None, dict[str, Any]]:
    """Load Gemini API credentials.

    If GEMINI_API_KEY is set in the environment, it returns (None, {}).
    Otherwise, on Windows, it retrieves the OAuth access token from the
    'gemini:antigravity' generic credential in Windows Credential Manager
    and returns a google.oauth2.credentials.Credentials object configured for
    Vertex AI.

    Returns:
        A tuple of (Credentials, language_model_params).
    """
    if os.getenv("GEMINI_API_KEY"):
        print(
            "GEMINI_API_KEY is set in the environment. Using standard API key auth."
        )
        return None, {}

    if os.name != "nt":
        print("Not running on Windows; standard API key auth required.")
        return None, {}

    print("Reading 'gemini:antigravity' from Windows Credential Manager...")
    try:
        from google.oauth2.credentials import Credentials

        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        cred_read_w = advapi32.CredReadW
        cred_read_w.argtypes = [
            wintypes.LPWSTR,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.POINTER(CREDENTIALW)),
        ]
        cred_read_w.restype = wintypes.BOOL

        cred_free = advapi32.CredFree
        cred_free.argtypes = [ctypes.c_void_p]
        cred_free.restype = None

        cred_type_generic = 1
        p_cred = ctypes.POINTER(CREDENTIALW)()

        ok = cred_read_w(
            "gemini:antigravity", cred_type_generic, 0, ctypes.byref(p_cred)
        )
        if not ok:
            err = ctypes.get_last_error()
            print(f"CredReadW failed with Windows error: {err}")
            return None, {}

        try:
            cred = p_cred.contents
            size = cred.CredentialBlobSize
            if not cred.CredentialBlob or size == 0:
                print("Credential blob is empty.")
                return None, {}

            blob_bytes = bytes(cred.CredentialBlob[:size])
            cred_json = json.loads(blob_bytes.decode("utf-8"))
            token_data = cred_json.get("token", {})
            access_token = token_data.get("access_token")

            if not access_token:
                print("OAuth access token not found inside credential blob.")
                return None, {}

            print("Successfully loaded Gemini access token for Vertex AI.")
            creds = Credentials(access_token)
            lm_params = {
                "vertexai": True,
                "project": "rgfr-8927d",
                "location": "us-central1",
                "credentials": creds,
            }
            return creds, lm_params
        finally:
            cred_free(p_cred)

    except Exception as e:  # pylint: disable=broad-except
        print(f"Failed to read credential from Credential Manager: {e}")
        return None, {}


def get_webview2_profile_dir() -> Path | None:
    """Get the WebView2 default profile directory.

    Returns:
        The directory path if it exists, or None.
    """
    appdata = os.getenv("LOCALAPPDATA")
    if not appdata:
        appdata = str(Path.home() / "AppData" / "Local")

    profile_dir = (
        Path(appdata)
        / "Google"
        / "Google"
        / "latest"
        / "default"
        / "WebView2"
        / "EBWebView"
        / "Default"
    )
    if profile_dir.exists():
        return profile_dir

    print(f"WebView2 Default profile directory does not exist: {profile_dir}")
    return None


def run_analysis() -> None:
    """Execute the WebView2 data analysis pipeline."""
    # 1. Load API credentials
    _, lm_params = get_gemini_credentials()

    # 2. Locate WebView2 data
    profile_dir = get_webview2_profile_dir()
    if not profile_dir:
        print("Exiting: WebView2 user profile not found.")
        return

    print(f"Found WebView2 user profile: {profile_dir}")

    # 3. Classify target files using Magika
    print("\nClassifying WebView2 profile files using Magika:")
    target_files = ["History", "Preferences", "Web Data"]
    magika = Magika()
    for fname in target_files:
        fpath = profile_dir / fname
        if fpath.exists():
            res = magika.identify_path(fpath)
            print(
                f"  - File: {fname:<12} -> Magika: {res.output.label} "
                f"(MIME: {res.output.mime_type})"
            )
        else:
            print(f"  - File: {fname:<12} -> Not Found")

    # 4. Extract entries from SQLite History database
    history_file = profile_dir / "History"
    if not history_file.exists():
        print("\nExiting: WebView2 History file not found.")
        return

    print("\nReading recent Visited URLs from History database...")
    history_entries: list[dict[str, Any]] = []

    with tempfile.TemporaryDirectory() as tmpdir:
        temp_db = Path(tmpdir) / "History_copy"
        shutil.copy2(history_file, temp_db)

        conn = sqlite3.connect(temp_db)
        try:
            cursor = conn.cursor()
            cursor.execute(
                "SELECT url, title, visit_count, last_visit_time FROM urls "
                "ORDER BY last_visit_time DESC LIMIT 15;"
            )
            for row in cursor.fetchall():
                history_entries.append(
                    {
                        "url": row[0],
                        "title": row[1],
                        "visit_count": row[2],
                        "last_visit_time": row[3],
                    }
                )
        except sqlite3.Error as e:
            print(f"Error querying SQLite database: {e}")
            return
        finally:
            conn.close()

    if not history_entries:
        print("No history entries found.")
        return

    print(f"Retrieved {len(history_entries)} history items.")

    # 5. Format history items into text for LangExtract
    document_lines = []
    for entry in history_entries:
        document_lines.append(f"URL: {entry['url']}")
        document_lines.append(f"Title: {entry['title']}")
        document_lines.append(f"Visit Count: {entry['visit_count']}")
        document_lines.append("---")
    document_text = "\n".join(document_lines)

    # 6. Set up LangExtract schema and prompt materials
    prompt = (
        "Extract search queries and visited websites in order of appearance.\n"
        "Identify search queries (SearchQuery class) and direct website visits "
        "(VisitedWebsite class).\n"
        "Extract exact URL or title where appropriate, and infer category/topic "
        "attributes."
    )

    examples = [
        lx.data.ExampleData(
            text=(
                "URL: https://www.google.com/search?q=gemini+api+docs\n"
                "Title: gemini api docs - Google Search\n"
                "Visit Count: 2\n"
                "---\n"
                "URL: https://ai.google.dev/gemini-api/docs\n"
                "Title: Gemini API Documentation | Google AI Studio\n"
                "Visit Count: 1\n"
                "---\n"
            ),
            extractions=[
                lx.data.Extraction(
                    extraction_class="SearchQuery",
                    extraction_text="gemini api docs - Google Search",
                    attributes={
                        "query": "gemini api docs",
                        "engine": "Google Search",
                        "topic": "developer documentation search",
                    },
                ),
                lx.data.Extraction(
                    extraction_class="VisitedWebsite",
                    extraction_text="Gemini API Documentation | Google AI Studio",
                    attributes={
                        "url": "https://ai.google.dev/gemini-api/docs",
                        "title": "Gemini API Documentation | Google AI Studio",
                        "category": "developer documentation",
                    },
                ),
            ],
        )
    ]

    print("\nRunning LangExtract structure synthesis using Gemini model...")
    try:
        result = lx.extract(
            text_or_documents=document_text,
            prompt_description=prompt,
            examples=examples,
            model_id="gemini-2.5-flash",
            language_model_params=lm_params if lm_params else None,
        )

        grounded_extractions = [
            e for e in result.extractions if e.char_interval is not None
        ]
        print(f"Extracted {len(grounded_extractions)} grounded entities:")
        for idx, ext in enumerate(grounded_extractions, 1):
            print(
                f"  {idx}. [{ext.extraction_class}] text='{ext.extraction_text}'"
            )
            if ext.attributes:
                print(f"     Attributes: {ext.attributes}")

        # 7. Save outputs
        output_jsonl = "webview2_analysis_results.jsonl"
        lx.io.save_annotated_documents(
            [result], output_name=output_jsonl, output_dir="."
        )
        print(f"\nSaved structured JSONL output to: {output_jsonl}")

        # 8. Generate interactive visualization
        html_content = lx.visualize(output_jsonl)
        output_html = "webview2_visualization.html"
        with open(output_html, "w", encoding="utf-8") as f:
            if hasattr(html_content, "data"):
                f.write(html_content.data)
            else:
                f.write(html_content)
        print(f"Saved interactive HTML visualization to: {output_html}")

    except Exception as e:  # pylint: disable=broad-except
        print(f"Failed during LangExtract processing: {e}")


if __name__ == "__main__":
    import sys

    if sys.platform.startswith("win"):
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    run_analysis()
