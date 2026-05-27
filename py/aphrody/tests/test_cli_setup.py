# SPDX-License-Identifier: Apache-2.0
"""Tests for the setup CLI command in :mod:`aphrody.cli.setup`."""

from __future__ import annotations

from pathlib import Path
from unittest import mock

import pytest
from aphrody.cli import Aphrody
from aphrody.cli.setup import setup_secrets


def test_cli_setup_success(tmp_path) -> None:
    # Set up temp repository structure
    git_dir = tmp_path / ".git"
    git_dir.mkdir()

    # We will mock subprocess.run to intercept gcloud commands
    def mock_run(args, **kwargs):
        cmd = " ".join(args)

        # Mock responses
        if "gcloud --version" in cmd:
            return mock.Mock(
                returncode=0, stdout="Google Cloud SDK 400.0.0", stderr=""
            )
        elif "keys create" in cmd:
            # Create a dummy key file to simulate gcloud writing it
            key_path = Path(args[5])
            key_path.parent.mkdir(parents=True, exist_ok=True)
            key_path.write_text("{}", encoding="utf-8")
            return mock.Mock(returncode=0, stdout="created key", stderr="")
        elif "activate-service-account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="Activated service account credentials",
                stderr="",
            )
        elif "config get-value account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="aphrody-bot@aphrody.iam.gserviceaccount.com",
                stderr="",
            )
        elif "add-iam-policy-binding" in cmd:
            return mock.Mock(returncode=0, stdout="bindings updated", stderr="")
        elif "services enable" in cmd:
            return mock.Mock(returncode=0, stdout="APIs enabled", stderr="")
        elif "print-access-token" in cmd:
            return mock.Mock(
                returncode=0, stdout="ya29.mock_token_12345", stderr=""
            )

        return mock.Mock(returncode=0, stdout="", stderr="")

    with (
        mock.patch("subprocess.run", side_effect=mock_run),
        mock.patch("time.sleep"),
    ):
        # We run it with our temporary repository root
        with mock.patch("pathlib.Path.cwd", return_value=tmp_path):
            success = setup_secrets(repo_root=tmp_path)
            assert success is True

            # Verify the key file was created
            key_file = tmp_path / "var" / "secrets" / "aphrody-bot-key.json"
            assert key_file.exists()

            # Verify .env file was created and contains correct data
            env_file = tmp_path / ".env"
            assert env_file.exists()
            env_content = env_file.read_text(encoding="utf-8")
            assert (
                "GOOGLE_APPLICATION_CREDENTIALS=var/secrets/aphrody-bot-key.json"
                in env_content
            )
            assert "APHRODY_SECRETS_DIR=var/secrets" in env_content


def test_cli_setup_command_exit_on_failure(tmp_path) -> None:
    # Test that the setup command exits with 1 when setup_secrets returns False
    cli = Aphrody()
    with mock.patch(
        "aphrody.cli.setup.setup_secrets", return_value=False
    ) as mock_setup:
        with pytest.raises(SystemExit) as excinfo:
            cli.setup()
        assert excinfo.value.code == 1
        mock_setup.assert_called_once()


def test_cli_setup_custom_params(tmp_path) -> None:
    git_dir = tmp_path / ".git"
    git_dir.mkdir()

    captured_cmds = []

    def mock_run(args, **kwargs):
        cmd = " ".join(args)
        captured_cmds.append(cmd)
        if "gcloud --version" in cmd:
            return mock.Mock(
                returncode=0, stdout="Google Cloud SDK 400.0.0", stderr=""
            )
        elif "keys create" in cmd:
            key_path = Path(args[5])
            key_path.parent.mkdir(parents=True, exist_ok=True)
            key_path.write_text("{}", encoding="utf-8")
            return mock.Mock(returncode=0, stdout="created key", stderr="")
        elif "activate-service-account" in cmd:
            return mock.Mock(returncode=0, stdout="Activated", stderr="")
        elif "config get-value account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="custom-sa@custom-proj.iam.gserviceaccount.com",
                stderr="",
            )
        elif "add-iam-policy-binding" in cmd:
            return mock.Mock(returncode=0, stdout="bindings updated", stderr="")
        elif "services enable" in cmd:
            return mock.Mock(returncode=0, stdout="APIs enabled", stderr="")
        elif "print-access-token" in cmd:
            return mock.Mock(
                returncode=0, stdout="ya29.mock_token_12345", stderr=""
            )
        return mock.Mock(returncode=0, stdout="", stderr="")

    with (
        mock.patch("subprocess.run", side_effect=mock_run),
        mock.patch("time.sleep"),
    ):
        with mock.patch("pathlib.Path.cwd", return_value=tmp_path):
            success = setup_secrets(
                repo_root=tmp_path,
                project_id="custom-proj",
                service_account="custom-sa@custom-proj.iam.gserviceaccount.com",
                location="europe-west9",
            )
            assert success is True

            key_file = tmp_path / "var" / "secrets" / "custom-sa-key.json"
            assert key_file.exists()

            env_file = tmp_path / ".env"
            assert env_file.exists()
            env_content = env_file.read_text(encoding="utf-8")
            assert (
                "GOOGLE_APPLICATION_CREDENTIALS=var/secrets/custom-sa-key.json"
                in env_content
            )
            assert "GOOGLE_CLOUD_PROJECT=custom-proj" in env_content
            assert "VERTEX_PROJECT=custom-proj" in env_content
            assert "APHRODY_VERTEX_PROJECT=custom-proj" in env_content
            assert "APHRODY_VERTEX_LOCATION=europe-west9" in env_content


def test_cli_setup_env_vars(tmp_path) -> None:
    git_dir = tmp_path / ".git"
    git_dir.mkdir()

    captured_cmds = []

    def mock_run(args, **kwargs):
        cmd = " ".join(args)
        captured_cmds.append(cmd)
        if "gcloud --version" in cmd:
            return mock.Mock(
                returncode=0, stdout="Google Cloud SDK 400.0.0", stderr=""
            )
        elif "keys create" in cmd:
            key_path = Path(args[5])
            key_path.parent.mkdir(parents=True, exist_ok=True)
            key_path.write_text("{}", encoding="utf-8")
            return mock.Mock(returncode=0, stdout="created key", stderr="")
        elif "activate-service-account" in cmd:
            return mock.Mock(returncode=0, stdout="Activated", stderr="")
        elif "config get-value account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="env-sa@env-proj.iam.gserviceaccount.com",
                stderr="",
            )
        elif "add-iam-policy-binding" in cmd:
            return mock.Mock(returncode=0, stdout="bindings updated", stderr="")
        elif "services enable" in cmd:
            return mock.Mock(returncode=0, stdout="APIs enabled", stderr="")
        elif "print-access-token" in cmd:
            return mock.Mock(
                returncode=0, stdout="ya29.mock_token_12345", stderr=""
            )
        return mock.Mock(returncode=0, stdout="", stderr="")

    mock_env = {
        "APHRODY_PROJECT_ID": "env-proj",
        "APHRODY_SERVICE_ACCOUNT": "env-sa@env-proj.iam.gserviceaccount.com",
        "APHRODY_LOCATION": "asia-east1",
    }

    with (
        mock.patch("subprocess.run", side_effect=mock_run),
        mock.patch("time.sleep"),
        mock.patch.dict("os.environ", mock_env),
    ):
        with mock.patch("pathlib.Path.cwd", return_value=tmp_path):
            success = setup_secrets(repo_root=tmp_path)
            assert success is True

            key_file = tmp_path / "var" / "secrets" / "env-sa-key.json"
            assert key_file.exists()

            env_file = tmp_path / ".env"
            assert env_file.exists()
            env_content = env_file.read_text(encoding="utf-8")
            assert (
                "GOOGLE_APPLICATION_CREDENTIALS=var/secrets/env-sa-key.json"
                in env_content
            )
            assert "GOOGLE_CLOUD_PROJECT=env-proj" in env_content
            assert "VERTEX_PROJECT=env-proj" in env_content
            assert "APHRODY_VERTEX_PROJECT=env-proj" in env_content
            assert "APHRODY_VERTEX_LOCATION=asia-east1" in env_content


def test_cli_setup_command_routing() -> None:
    cli = Aphrody()
    with mock.patch(
        "aphrody.cli.setup.setup_secrets", return_value=True
    ) as mock_setup:
        cli.setup(
            project="cli-proj",
            service_account="cli-sa",
            location="us-east4",
            interactive=True,
        )
        mock_setup.assert_called_once_with(
            project_id="cli-proj",
            service_account="cli-sa",
            location="us-east4",
            interactive=True,
        )


def test_cli_setup_interactive(tmp_path) -> None:
    # Set up temp repository structure
    git_dir = tmp_path / ".git"
    git_dir.mkdir()

    # Inputs to mock: project_id, service_account, location
    mock_inputs = [
        "my-custom-project",
        "my-custom-sa@my-custom-project.iam.gserviceaccount.com",
        "europe-west1",
    ]

    captured_cmds = []

    def mock_run(args, **kwargs):
        cmd = " ".join(args)
        captured_cmds.append(cmd)

        if "gcloud --version" in cmd:
            return mock.Mock(
                returncode=0, stdout="Google Cloud SDK 400.0.0", stderr=""
            )
        elif "keys create" in cmd:
            # Create a dummy key file to simulate gcloud writing it
            key_path = Path(args[5])
            key_path.parent.mkdir(parents=True, exist_ok=True)
            key_path.write_text("{}", encoding="utf-8")
            return mock.Mock(returncode=0, stdout="created key", stderr="")
        elif "activate-service-account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="Activated service account credentials",
                stderr="",
            )
        elif "config get-value account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="my-custom-sa@my-custom-project.iam.gserviceaccount.com",
                stderr="",
            )
        elif "add-iam-policy-binding" in cmd:
            return mock.Mock(returncode=0, stdout="bindings updated", stderr="")
        elif "services enable" in cmd:
            return mock.Mock(returncode=0, stdout="APIs enabled", stderr="")
        elif "print-access-token" in cmd:
            return mock.Mock(
                returncode=0, stdout="ya29.mock_token_12345", stderr=""
            )

        return mock.Mock(returncode=0, stdout="", stderr="")

    with (
        mock.patch("subprocess.run", side_effect=mock_run),
        mock.patch("time.sleep"),
        mock.patch("builtins.input", side_effect=mock_inputs),
    ):
        with mock.patch("pathlib.Path.cwd", return_value=tmp_path):
            success = setup_secrets(repo_root=tmp_path, interactive=True)
            assert success is True

            # Verify key file was named after the custom service account username
            key_file = tmp_path / "var" / "secrets" / "my-custom-sa-key.json"
            assert key_file.exists()

            # Verify .env file was created and contains correct custom data
            env_file = tmp_path / ".env"
            assert env_file.exists()
            env_content = env_file.read_text(encoding="utf-8")
            assert (
                "GOOGLE_APPLICATION_CREDENTIALS=var/secrets/my-custom-sa-key.json"
                in env_content
            )
            assert "GOOGLE_CLOUD_PROJECT=my-custom-project" in env_content
            assert "VERTEX_PROJECT=my-custom-project" in env_content
            assert "APHRODY_VERTEX_PROJECT=my-custom-project" in env_content
            assert "APHRODY_VERTEX_LOCATION=europe-west1" in env_content

            # Verify commands used the custom arguments
            assert any(
                "keys create" in cmd
                and "--iam-account=my-custom-sa@my-custom-project.iam.gserviceaccount.com"
                in cmd
                and "--project=my-custom-project" in cmd
                for cmd in captured_cmds
            )
            assert any(
                "activate-service-account my-custom-sa@my-custom-project.iam.gserviceaccount.com"
                in cmd
                and "--project=my-custom-project" in cmd
                for cmd in captured_cmds
            )
            assert any(
                "add-iam-policy-binding my-custom-project" in cmd
                and "--member=serviceAccount:my-custom-sa@my-custom-project.iam.gserviceaccount.com"
                in cmd
                for cmd in captured_cmds
            )


def test_cli_setup_interactive_defaults(tmp_path) -> None:
    git_dir = tmp_path / ".git"
    git_dir.mkdir()

    # Inputs are empty strings (pressing Enter for each prompt)
    mock_inputs = ["", "", ""]

    captured_cmds = []

    def mock_run(args, **kwargs):
        cmd = " ".join(args)
        captured_cmds.append(cmd)

        if "gcloud --version" in cmd:
            return mock.Mock(
                returncode=0, stdout="Google Cloud SDK 400.0.0", stderr=""
            )
        elif "keys create" in cmd:
            key_path = Path(args[5])
            key_path.parent.mkdir(parents=True, exist_ok=True)
            key_path.write_text("{}", encoding="utf-8")
            return mock.Mock(returncode=0, stdout="created key", stderr="")
        elif "activate-service-account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="Activated service account credentials",
                stderr="",
            )
        elif "config get-value account" in cmd:
            return mock.Mock(
                returncode=0,
                stdout="aphrody-bot@aphrody.iam.gserviceaccount.com",
                stderr="",
            )
        elif "add-iam-policy-binding" in cmd:
            return mock.Mock(returncode=0, stdout="bindings updated", stderr="")
        elif "services enable" in cmd:
            return mock.Mock(returncode=0, stdout="APIs enabled", stderr="")
        elif "print-access-token" in cmd:
            return mock.Mock(
                returncode=0, stdout="ya29.mock_token_12345", stderr=""
            )

        return mock.Mock(returncode=0, stdout="", stderr="")

    import os

    clean_env = os.environ.copy()
    for var in [
        "GOOGLE_CLOUD_PROJECT",
        "APHRODY_PROJECT_ID",
        "APHRODY_SERVICE_ACCOUNT",
        "APHRODY_LOCATION",
        "APHRODY_VERTEX_LOCATION",
    ]:
        clean_env.pop(var, None)

    with (
        mock.patch("subprocess.run", side_effect=mock_run),
        mock.patch("time.sleep"),
        mock.patch("builtins.input", side_effect=mock_inputs),
        mock.patch.dict("os.environ", clean_env, clear=True),
    ):
        with mock.patch("pathlib.Path.cwd", return_value=tmp_path):
            success = setup_secrets(repo_root=tmp_path, interactive=True)
            assert success is True

            # Verify key file was named after the default service account username
            key_file = tmp_path / "var" / "secrets" / "aphrody-bot-key.json"
            assert key_file.exists()

            # Verify .env file was created and contains correct default data
            env_file = tmp_path / ".env"
            assert env_file.exists()
            env_content = env_file.read_text(encoding="utf-8")
            assert (
                "GOOGLE_APPLICATION_CREDENTIALS=var/secrets/aphrody-bot-key.json"
                in env_content
            )
            assert "GOOGLE_CLOUD_PROJECT=aphrody" in env_content
            assert "VERTEX_PROJECT=aphrody" in env_content
            assert "APHRODY_VERTEX_PROJECT=aphrody" in env_content
            assert "APHRODY_VERTEX_LOCATION=us-central1" in env_content


def test_cli_setup_command_interactive(tmp_path) -> None:
    cli = Aphrody()
    with mock.patch(
        "aphrody.cli.setup.setup_secrets", return_value=True
    ) as mock_setup:
        cli.setup(interactive=True)
        mock_setup.assert_called_once_with(
            project_id=None,
            service_account=None,
            location=None,
            interactive=True,
        )


def test_cli_setup_command_i_alias(tmp_path) -> None:
    cli = Aphrody()
    with mock.patch(
        "aphrody.cli.setup.setup_secrets", return_value=True
    ) as mock_setup:
        cli.setup(i=True)
        mock_setup.assert_called_once_with(
            project_id=None,
            service_account=None,
            location=None,
            interactive=True,
        )
