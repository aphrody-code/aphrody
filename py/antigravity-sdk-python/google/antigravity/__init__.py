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

"""Google Antigravity SDK for building AI agents."""

from typing import TYPE_CHECKING, Any

from google.antigravity.agent import Agent
from google.antigravity.connections.connection import AgentConfig
from google.antigravity.connections.local.local_connection_config import (
    LocalAgentConfig,
)
from google.antigravity.tools.tool_context import ToolContext
from google.antigravity.types import (
    CapabilitiesConfig,
    GeminiConfig,
    GenerationConfig,
    ModelConfig,
    ModelEntry,
    ThinkingLevel,
    UsageMetadata,
)

if TYPE_CHECKING:
    from google.antigravity.voice import (
        LocalKokoroTextToSpeech,
        LocalVoiceAgentLoop,
        LocalWhisperSpeechToText,
        SpeechToText,
        TextToSpeech,
    )

__all__ = [
    "Agent",
    "AgentConfig",
    "CapabilitiesConfig",
    "GeminiConfig",
    "GenerationConfig",
    "LocalAgentConfig",
    "LocalKokoroTextToSpeech",
    "LocalVoiceAgentLoop",
    "LocalWhisperSpeechToText",
    "ModelConfig",
    "ModelEntry",
    "SpeechToText",
    "TextToSpeech",
    "ThinkingLevel",
    "ToolContext",
    "UsageMetadata",
]


def __getattr__(name: str) -> Any:
    if name in {
        "LocalVoiceAgentLoop",
        "LocalWhisperSpeechToText",
        "LocalKokoroTextToSpeech",
        "SpeechToText",
        "TextToSpeech",
    }:
        from google.antigravity import voice

        return getattr(voice, name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
