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

"""Configuration for the local harness connection strategy."""

import logging
import os
import pathlib
import tempfile
from typing import Any

import pydantic
from google.antigravity import types
from google.antigravity.connections import connection
from google.antigravity.hooks import policy

DEFAULT_APP_DATA_DIR = (
    pathlib.Path("~/.gemini/antigravity").expanduser().resolve()
)


class LocalAgentConfig(connection.AgentConfig):
    """Configuration for the local harness backend.

    This is the default config for the Agent class. It uses the
    Go-based localharness binary.

    By default, all tools are enabled but ``run_command`` is denied via
    ``policy.confirm_run_command()``.  To enable fully autonomous execution
    (including shell access), pass ``policies=[policy.allow_all()]``.

    When ``workspaces`` are configured, file tools are automatically
    restricted to those directories via ``policy.workspace_only()``.
    """

    capabilities: types.CapabilitiesConfig = pydantic.Field(
        default_factory=types.CapabilitiesConfig
    )
    policies: list[Any] = pydantic.Field(
        default_factory=policy.confirm_run_command
    )
    workspaces: list[str] = pydantic.Field(
        default_factory=lambda: [os.getcwd()]
    )

    gemini_config: types.GeminiConfig = pydantic.Field(
        default_factory=types.GeminiConfig
    )

    # Top-level shorthand fields — flow into gemini_config.
    model: str | None = None
    api_key: str | None = None

    @pydantic.field_validator("app_data_dir")
    @classmethod
    def _validate_app_data_dir(cls, v: str | None) -> str | None:
        if v is not None and not os.path.isabs(v):
            raise ValueError(
                f"app_data_dir must be an absolute path, got '{v}'"
            )
        return v

    @pydantic.model_validator(mode="after")
    def _apply_shorthand_configs(self) -> "LocalAgentConfig":
        """Applies top-level shorthand fields (model, api_key) to gemini_config."""
        # Defensive copy: prevent mutation of shared GeminiConfig instances.
        self.gemini_config = self.gemini_config.model_copy(deep=True)

        if self.model is not None:
            if "default" in self.gemini_config.models.model_fields_set:
                raise ValueError(
                    "Cannot set both 'model' shorthand and "
                    "'gemini_config.models.default'. Use one or the other."
                )
            self.gemini_config.models.default = types.ModelEntry(
                name=self.model
            )
        if self.api_key is not None:
            if self.gemini_config.api_key is not None:
                raise ValueError(
                    "Cannot set both 'api_key' shorthand and "
                    "'gemini_config.api_key'. Use one or the other."
                )
            self.gemini_config.api_key = self.api_key
        return self

    @pydantic.model_validator(mode="after")
    def _apply_workspace_policies(self) -> "LocalAgentConfig":
        """Prepends workspace-scoping policies when workspaces are configured.

        Always prepends — even when the user sets explicit policies — so that
        file operations are always restricted to the configured workspaces.
        Users who want truly unrestricted access should set ``workspaces=[]``.
        """
        if self.workspaces:
            # Automatically include the app data directory in the workspace allowlist
            app_data_path = self.app_data_dir or DEFAULT_APP_DATA_DIR
            resolved_app_data_dir = (
                pathlib.Path(app_data_path).expanduser().resolve()
            )
            allowed_paths = [*self.workspaces, str(resolved_app_data_dir)]

            self.policies = policy.workspace_only(allowed_paths) + self.policies
        return self

    def create_strategy(
        self,
        *,
        tool_runner: Any,
        hook_runner: Any,
    ) -> "connection.ConnectionStrategy":
        """Creates a ConnectionStrategy instance based on this configuration."""
        # Late import to avoid circular dependency: local_connection.py imports
        # this config module, so we import the strategy class here at call time.
        from google.antigravity.connections.local import (
            local_connection,  # pylint: disable=g-import-not-at-top
        )

        if isinstance(self.system_instructions, str):
            si = types.TemplatedSystemInstructions(
                sections=[
                    types.SystemInstructionSection(
                        content=self.system_instructions
                    )
                ]
            )
        else:
            si = self.system_instructions

        save_dir = self.save_dir
        if save_dir is None:
            save_dir = tempfile.mkdtemp(prefix="antigravity_")
            logging.info("No save_dir specified; using %s", save_dir)

        return local_connection.LocalConnectionStrategy(
            tool_runner=tool_runner,
            hook_runner=hook_runner,
            gemini_config=self.gemini_config,
            system_instructions=si,
            capabilities_config=self.capabilities,
            conversation_id=self.conversation_id,
            save_dir=save_dir,
            workspaces=self.workspaces,
            app_data_dir=self.app_data_dir,
            skills_paths=self.skills_paths,
        )
