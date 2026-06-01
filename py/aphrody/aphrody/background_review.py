# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Background self-improvement reviews curation system."""

import json
import logging
import re
import threading
import time
from pathlib import Path
from typing import Any

class BxcGeminiWebClient:
    """Helper client that calls the migrated bxc CLI tool under the hood."""
    def generate(self, prompt: str) -> str:
        import subprocess
        from pathlib import Path
        import os
        paths = [
            "bxc",
            str(Path(os.path.expanduser("~/.local/bin/bxc"))),
            "/usr/local/bin/bxc",
        ]
        last_err = None
        for p in paths:
            try:
                res = subprocess.run([p, "google", "chat", prompt], capture_output=True, text=True, check=True)
                return res.stdout
            except Exception as e:
                last_err = e
                continue
        raise RuntimeError(f"Failed to run bxc CLI: {last_err}")

    def close(self) -> None:
        pass

from aphrody.session_db import SessionDB

logger = logging.getLogger(__name__)

DEFAULT_MEMORY_FILE = (
    Path(__file__).resolve().parents[2] / "var" / "secrets" / "memory.md"
)
DEFAULT_SKILLS_DIR = (
    Path(__file__).resolve().parents[2] / "var" / "secrets" / "skills"
)


class BackgroundReview:
    """Daemon task that runs post-session self-improvement reviews."""

    def __init__(
        self,
        db_path: Path | None = None,
        memory_file: Path | None = None,
        skills_dir: Path | None = None,
    ):
        self.db_path = db_path
        self.memory_file = memory_file or DEFAULT_MEMORY_FILE
        self.skills_dir = skills_dir or DEFAULT_SKILLS_DIR

    def run_review(
        self, session_id: str, client: Any | None = None
    ) -> dict[str, Any]:
        """Load the session history, send it to Gemini for review.

        Retrieves all messages for the session, constructs a prompt,
        invokes the Gemini client, and applies updates to the local
        memory and skill files.
        """
        logger.info("Starting background review for session %s", session_id)
        db = SessionDB(self.db_path)
        try:
            messages = db.get_messages(session_id)
        finally:
            db.close()

        if not messages:
            logger.warning(
                "No messages found for session %s. Skipping review.", session_id
            )
            return {}

        # Format conversation history
        history_text = []
        for msg in messages:
            role = msg.get("role", "unknown")
            content = msg.get("content") or ""
            if msg.get("tool_calls"):
                content += f" [Tool Calls: {msg['tool_calls']}]"
            if msg.get("tool_name"):
                content += f" [Tool Name: {msg['tool_name']}]"
            history_text.append(f"{role.upper()}: {content}")

        history_str = "\n".join(history_text)

        # Build prompt
        prompt = f"""You are a background self-improvement curator for the Antigravity assistant.
Review the following conversation history between the User and the Assistant:

<CONVERSATION_HISTORY>
{history_str}
</CONVERSATION_HISTORY>

Analyze this session to extract:
1. Durable facts about the user (preferences, expectations, style corrections) to update their persona memory.
2. Procedural rules/instructions (workarounds, debug steps, style constraints) to update the agent's directives.

Format your response as a valid JSON object matching this schema:
{{
  "memory_update": "Markdown content detailing new facts/preferences about the user, or null if nothing to save.",
  "skills_update": {{
     "skill_name": "Name of the skill category (e.g. build, deploy, test)",
     "file_path": "Recommended filename (e.g. build.md)",
     "content": "Markdown content containing the updated rules/instructions for this class of task, or null if nothing to save."
  }}
}}

Ensure that the output is ONLY valid JSON.
"""

        # Call Gemini (defaulting to keyless web client if not provided)
        local_client = False
        if client is None:
            try:
                client = BxcGeminiWebClient()
                local_client = True
            except Exception as e:
                logger.error(
                    "Failed to load BxcGeminiWebClient for background review: %s",
                    e,
                )
                return {}

        try:
            response_text = client.generate(prompt)
        except Exception as e:
            logger.error("Failed to generate background review response: %s", e)
            return {}
        finally:
            if local_client:
                try:
                    client.close()
                except Exception:
                    pass

        # Parse output
        updates = self._parse_json_response(response_text)
        if not updates:
            logger.warning(
                "Could not parse background review response as JSON."
            )
            return {}

        # Apply updates
        self._apply_updates(updates)
        return updates

    def _parse_json_response(self, text: str) -> dict[str, Any] | None:
        # Strip markdown code blocks if present
        clean_text = text.strip()
        match = re.search(
            r"```(?:json)?\s*([\s\S]*?)```", clean_text, re.IGNORECASE
        )
        if match:
            clean_text = match.group(1).strip()
        try:
            return json.loads(clean_text)
        except json.JSONDecodeError:
            # Try finding any JSON object block
            obj_match = re.search(r"\{[\s\S]*\}", clean_text)
            if obj_match:
                try:
                    return json.loads(obj_match.group(0))
                except json.JSONDecodeError:
                    pass
        return None

    def _apply_updates(self, updates: dict[str, Any]) -> None:
        # 1. Apply memory update
        mem_update = updates.get("memory_update")
        if mem_update and mem_update.strip():
            self.memory_file.parent.mkdir(parents=True, exist_ok=True)
            with open(self.memory_file, "a", encoding="utf-8") as f:
                f.write(
                    f"\n# Session Update ({time.strftime('%Y-%m-%d %H:%M:%S')})\n"
                )
                f.write(mem_update.strip() + "\n")
            logger.info(
                "Memory update applied successfully to %s", self.memory_file
            )

        # 2. Apply skills/directives update
        skills_update = updates.get("skills_update")
        if skills_update and isinstance(skills_update, dict):
            content = skills_update.get("content")
            file_path_str = skills_update.get("file_path")
            if content and content.strip() and file_path_str:
                # Sanitize path to prevent directory traversal
                safe_name = Path(file_path_str).name
                if not safe_name.endswith(".md"):
                    safe_name += ".md"
                target_file = self.skills_dir / safe_name
                self.skills_dir.mkdir(parents=True, exist_ok=True)

                mode = "a" if target_file.exists() else "w"
                with open(target_file, mode, encoding="utf-8") as f:
                    if mode == "a":
                        f.write(
                            f"\n# Session Update ({time.strftime('%Y-%m-%d %H:%M:%S')})\n"
                        )
                    else:
                        f.write(
                            f"# Skill: {skills_update.get('skill_name', 'General')}\n"
                        )
                    f.write(content.strip() + "\n")
                logger.info(
                    "Skill update applied successfully to %s", target_file
                )


def spawn_background_review(
    session_id: str,
    db_path: Path | None = None,
    memory_file: Path | None = None,
    skills_dir: Path | None = None,
    client: Any | None = None,
) -> threading.Thread:
    """Spawns a daemon thread to run the background self-improvement review."""
    reviewer = BackgroundReview(
        db_path=db_path,
        memory_file=memory_file,
        skills_dir=skills_dir,
    )

    def _thread_target():
        try:
            reviewer.run_review(session_id, client=client)
        except Exception as e:
            logger.error("Background review thread execution failed: %s", e)

    thread = threading.Thread(target=_thread_target, daemon=True)
    thread.start()
    return thread
