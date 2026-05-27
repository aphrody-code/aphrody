# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Google Drive commands for the aphrody CLI."""

from __future__ import annotations

from aphrody.cli.utils import _emit


class DriveCommands:
    """``aphrody drive <action>`` — authenticated Google Drive workspace.

    Manage files, directories, assets, and research uploads on Google Drive
    using local client authentication.
    """

    def folder(self, name: str) -> None:
        """Find the ID of a folder, or create it if it does not exist.

        Args:
            name: Folder name.
        """
        from aphrody.google_drive import AuthenticatedDriveClient

        with AuthenticatedDriveClient() as client:
            folder_id = client.resolve_or_create_folder(name)
            _emit({"folder": name, "id": folder_id})

    def upload(self, file: str, folder: str | None = None) -> None:
        """Upload a local file to a Google Drive folder.

        Args:
            file: Local path of the file to upload.
            folder: Optional folder name or ID in Google Drive. If a name is
                given, it is automatically resolved or created.
        """
        from aphrody.google_drive import AuthenticatedDriveClient

        with AuthenticatedDriveClient() as client:
            folder_id = None
            if folder:
                # If folder looks like a name (not an ID), resolve or create it
                if len(folder) < 20 or "-" in folder or " " in folder:
                    folder_id = client.resolve_or_create_folder(folder)
                else:
                    folder_id = folder
            res = client.upload_file(file, folder_id=folder_id)
            _emit(res)

    def list(self, folder: str | None = None) -> None:
        """List files in Google Drive.

        Args:
            folder: Optional folder name or ID to filter files.
        """
        from aphrody.google_drive import AuthenticatedDriveClient

        with AuthenticatedDriveClient() as client:
            folder_id = None
            if folder:
                if len(folder) < 20 or "-" in folder or " " in folder:
                    folder_id = client.get_folder_id(folder)
                else:
                    folder_id = folder
            files = client.list_files(folder_id=folder_id)
            _emit(files)
