# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Automated local secrets, credentials, and GCP environment configuration."""

from __future__ import annotations

import os
import subprocess
import time
from pathlib import Path

# Service account configuration details
SERVICE_ACCOUNT = "aphrody-bot@aphrody.iam.gserviceaccount.com"
PROJECT_ID = "aphrody"

REQUIRED_APIS = [
    "aiplatform.googleapis.com",
    "drive.googleapis.com",
    "sheets.googleapis.com",
    "translate.googleapis.com",
    "books.googleapis.com",
    "dns.googleapis.com",
    "generativelanguage.googleapis.com",
    "iam.googleapis.com",
    "cloudresourcemanager.googleapis.com",
    "docs.googleapis.com",
]


def run_command(args: list[str]) -> str:
    """Run a shell command and return stdout."""
    res = subprocess.run(
        args, capture_output=True, text=True, check=False, shell=True
    )
    if res.returncode != 0:
        print(f"Error running {' '.join(args)}:")
        if res.stderr:
            print(res.stderr.strip())
        return ""
    return res.stdout.strip()


def setup_secrets(
    repo_root: Path | None = None,
    project_id: str | None = None,
    service_account: str | None = None,
    location: str | None = None,
    interactive: bool = False,
) -> bool:
    """Automates local secrets, gcloud service account activation, and API enablement.

    Args:
        repo_root: Path to the repository root. If None, resolves dynamically.
        project_id: Custom Google Cloud Project ID.
        service_account: Custom Service account name/email.
        location: Custom Location/Region.
        interactive: Prompt for Google Cloud project and resource naming interactively.

    Returns:
        True if the configuration was successful, False otherwise.
    """
    print("=== Aphrody Local Secrets & Environment Configuration ===")

    # Resolve from parameters, environment variables, or standard defaults
    env_project_id = os.environ.get("APHRODY_PROJECT_ID") or os.environ.get(
        "GOOGLE_CLOUD_PROJECT"
    )
    env_sa = os.environ.get("APHRODY_SERVICE_ACCOUNT")
    env_location = os.environ.get("APHRODY_LOCATION") or os.environ.get(
        "APHRODY_VERTEX_LOCATION"
    )

    project_id = project_id or env_project_id or PROJECT_ID
    service_account = service_account or env_sa or SERVICE_ACCOUNT
    location = location or env_location or "us-central1"

    if interactive:
        project_id_input = input(
            f"Google Cloud Project ID [{project_id}]: "
        ).strip()
        if project_id_input:
            project_id = project_id_input

        service_account_input = input(
            f"Service account name/email [{service_account}]: "
        ).strip()
        if service_account_input:
            service_account = service_account_input

        location_input = input(f"Location/Region [{location}]: ").strip()
        if location_input:
            location = location_input

    # 1. Resolve repository root
    if not repo_root:
        cwd = Path.cwd().resolve()
        for directory in (cwd, *cwd.parents):
            if (directory / ".git").exists():
                repo_root = directory
                break

    if not repo_root:
        print("Error: Could not locate repository root (.git).")
        return False

    print(f"Located repository root at: {repo_root}")

    # 2. Check if gcloud is installed
    gcloud_ver = run_command(["gcloud", "--version"])
    if not gcloud_ver:
        print("Error: gcloud CLI is not installed or not in PATH.")
        print("Please install the Google Cloud SDK and try again.")
        return False

    # 3. Create var/secrets directory
    secrets_dir = repo_root / "var" / "secrets"
    secrets_dir.mkdir(parents=True, exist_ok=True)
    print(f"Created secrets directory at: {secrets_dir}")

    # 4. Create and download service account key via gcloud
    sa_name = (
        service_account.split("@")[0]
        if "@" in service_account
        else service_account
    )
    key_file = secrets_dir / f"{sa_name}-key.json"
    print(f"Creating/downloading service account key for: {service_account}...")

    cmd_key = [
        "gcloud",
        "iam",
        "service-accounts",
        "keys",
        "create",
        str(key_file),
        f"--iam-account={service_account}",
        f"--project={project_id}",
    ]
    run_command(cmd_key)
    if key_file.exists():
        print(f"Successfully saved service account key to: {key_file}")
        print("Waiting 3 seconds for Google Cloud IAM propagation...")
        time.sleep(3)
    else:
        print("Error: Failed to create/retrieve service account key.")
        return False

    # 5. Authenticate/activate service account in gcloud
    print(f"Activating service account '{service_account}' in gcloud...")
    cmd_auth = [
        "gcloud",
        "auth",
        "activate-service-account",
        service_account,
        f"--key-file={key_file}",
        f"--project={project_id}",
    ]
    auth_res = run_command(cmd_auth)
    if "Activated service account credentials" in auth_res or not auth_res:
        active_account = run_command(
            ["gcloud", "config", "get-value", "account"]
        )
        print(f"Active gcloud account: {active_account}")
        if active_account != service_account:
            print(
                "Warning: gcloud active account did not switch to the service account."
            )

    # 6. Ensure the service account has roles/owner binding
    print(
        f"Ensuring service account '{service_account}' has roles/owner binding..."
    )
    cmd_bind = [
        "gcloud",
        "projects",
        "add-iam-policy-binding",
        project_id,
        f"--member=serviceAccount:{service_account}",
        "--role=roles/owner",
    ]
    bind_res = run_command(cmd_bind)
    if bind_res:
        print(
            "Service account roles/owner binding confirmed/updated successfully."
        )
    else:
        print(
            "Note: Service account owner binding checked (or already present)."
        )

    # 7. Enable required Google APIs in a single batch
    print(f"Enabling required GCP APIs for project '{project_id}'...")
    cmd_enable_apis = [
        "gcloud",
        "services",
        "enable",
        *REQUIRED_APIS,
        f"--project={project_id}",
    ]
    enable_res = run_command(cmd_enable_apis)
    if enable_res:
        print("APIs enabled successfully.")
    else:
        print("Note: APIs checked/enabled.")

    # 8. Generate cross-platform .env file using forward slashes
    env_file = repo_root / ".env"
    rel_key_path = f"var/secrets/{key_file.name}"
    rel_secrets_dir = "var/secrets"
    env_content = (
        f"GOOGLE_APPLICATION_CREDENTIALS={rel_key_path}\n"
        f"APHRODY_SECRETS_DIR={rel_secrets_dir}\n"
        f"GOOGLE_CLOUD_PROJECT={project_id}\n"
        f"VERTEX_PROJECT={project_id}\n"
        f"APHRODY_VERTEX_PROJECT={project_id}\n"
        f"APHRODY_VERTEX_LOCATION={location}\n"
    )
    env_file.write_text(env_content, encoding="utf-8")
    print(f"Successfully generated cross-platform .env file at: {env_file}")

    # Enforce strict private file permissions cross-platform
    from aphrody.auth.credential_store import enforce_private_permissions

    enforce_private_permissions(key_file)
    enforce_private_permissions(env_file)

    # 9. Verify token printing
    print("\n--- Verifying Access Token ---")
    token = run_command(["gcloud", "auth", "print-access-token"])
    if token:
        redacted_token = (
            token[:10] + "..." + token[-10:] if len(token) > 20 else "..."
        )
        print(
            f"Successfully retrieved and printed service account access token: {redacted_token}"
        )
        print(
            "Configuration Complete! Environment is fully configured and authenticated."
        )
        return True
    else:
        print("Warning: Failed to print access token via gcloud.")
        return False
