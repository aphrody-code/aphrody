# SPDX-License-Identifier: Apache-2.0
"""Tests for the Google Drive CLI commands in :mod:`aphrody.cli.drive`."""

from __future__ import annotations

from unittest import mock

from aphrody.cli import Aphrody


def test_cli_drive_folder() -> None:
    cli = Aphrody()
    with mock.patch(
        "aphrody.google_drive.AuthenticatedDriveClient"
    ) as mock_client_class:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.resolve_or_create_folder.return_value = "folder123"

        with mock.patch("aphrody.cli.drive._emit") as mock_emit:
            cli.drive().folder("test-folder")
            mock_client.resolve_or_create_folder.assert_called_once_with(
                "test-folder"
            )
            mock_emit.assert_called_once_with(
                {"folder": "test-folder", "id": "folder123"}
            )


def test_cli_drive_upload() -> None:
    cli = Aphrody()
    with mock.patch(
        "aphrody.google_drive.AuthenticatedDriveClient"
    ) as mock_client_class:
        mock_client = mock.MagicMock()
        mock_client_class.return_value.__enter__.return_value = mock_client
        mock_client.resolve_or_create_folder.return_value = "folder123"
        mock_client.upload_file.return_value = {"id": "file123"}

        with mock.patch("aphrody.cli.drive._emit") as mock_emit:
            cli.drive().upload("hello.txt", folder="my-folder")
            mock_client.resolve_or_create_folder.assert_called_once_with(
                "my-folder"
            )
            mock_client.upload_file.assert_called_once_with(
                "hello.txt", folder_id="folder123"
            )
            mock_emit.assert_called_once_with({"id": "file123"})
