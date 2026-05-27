# SPDX-License-Identifier: Apache-2.0
"""Tests for AuthenticatedDriveClient features."""

from __future__ import annotations

from unittest import mock

import pytest
from aphrody.google_drive import AuthenticatedDriveClient


@pytest.fixture(autouse=True)
def mock_access_token():
    with mock.patch(
        "aphrody.auth.credentials.access_token",
        return_value="fake_access_token",
    ):
        yield


def test_get_folder_id(httpx_mock) -> None:
    httpx_mock.add_response(
        url="https://www.googleapis.com/drive/v3/files?q=name+%3D+%27test-folder%27+and+mimeType+%3D+%27application%2Fvnd.google-apps.folder%27+and+trashed+%3D+false&fields=files%28id%2C+name%29",
        json={"files": [{"id": "folder123", "name": "test-folder"}]},
    )
    with AuthenticatedDriveClient() as client:
        assert client.get_folder_id("test-folder") == "folder123"


def test_get_folder_id_none(httpx_mock) -> None:
    httpx_mock.add_response(
        url="https://www.googleapis.com/drive/v3/files?q=name+%3D+%27test-folder%27+and+mimeType+%3D+%27application%2Fvnd.google-apps.folder%27+and+trashed+%3D+false&fields=files%28id%2C+name%29",
        json={"files": []},
    )
    with AuthenticatedDriveClient() as client:
        assert client.get_folder_id("test-folder") is None


def test_create_folder(httpx_mock) -> None:
    httpx_mock.add_response(
        method="POST",
        url="https://www.googleapis.com/drive/v3/files",
        json={"id": "newfolder123"},
    )
    with AuthenticatedDriveClient() as client:
        assert (
            client.create_folder("new-folder", parent_id="parent123")
            == "newfolder123"
        )


def test_upload_file(httpx_mock, tmp_path) -> None:
    fake_file = tmp_path / "hello.txt"
    fake_file.write_text("file content", encoding="utf-8")

    httpx_mock.add_response(
        method="POST",
        url="https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart",
        json={"id": "file123", "name": "hello.txt"},
    )
    with AuthenticatedDriveClient() as client:
        res = client.upload_file(fake_file, folder_id="folder123")
        assert res["id"] == "file123"
        assert res["name"] == "hello.txt"
