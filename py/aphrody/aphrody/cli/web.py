# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Gemini Web app command group for the aphrody CLI."""

from __future__ import annotations

from aphrody import gemini_web
from aphrody.cli.utils import _emit


class WebCommands:
    """``aphrody web <action>`` — ask and manage conversations in Gemini Web."""

    def __call__(
        self,
        prompt: str | None = None,
        *,
        cid: str | None = None,
        rid: str | None = None,
        rcid: str | None = None,
        thread: bool = False,
        repl: bool = False,
        model: str = "flash",
    ) -> None:
        """Ask the Gemini *web app* (gemini.google.com) via session cookies.

        The keyless cookie path: the same Boq backend the browser talks to,
        with no API key and no OAuth token — only the stored Google cookies.

        Args:
            prompt: The message to send. If omitted, starts an interactive REPL session.
            cid: Optional conversation ID to resume a thread.
            rid: Optional response ID to resume a thread.
            rcid: Optional candidate response ID to resume a thread.
            thread: When True, return a JSON response containing the text, the
                new conversation IDs, and the conversation title if available.
            repl: Force starting an interactive REPL session even if prompt is provided first.
            model: Select Gemini model ("flash", "flash-lite", "pro").
        """
        with gemini_web.GeminiWebClient(model=model) as client:
            if cid:
                client.resume(cid, rid, rcid)

            if prompt:
                if repl:
                    # Execute prompt first, then enter REPL
                    print("Gemini: ", end="", flush=True)
                    reply = client.generate(prompt, keep_context=True)
                    print(reply)
                else:
                    # Single-shot
                    if thread:
                        reply = client.generate(prompt, keep_context=True)
                        cid_out, rid_out, rcid_out = client.conversation
                        _emit(
                            {
                                "reply": reply,
                                "title": client.last_title,
                                "conversation": {
                                    "cid": cid_out,
                                    "rid": rid_out,
                                    "rcid": rcid_out,
                                },
                            }
                        )
                    else:
                        _emit(client.generate(prompt, keep_context=False))
                    return

            # Interactive REPL
            print(
                "Starting interactive Gemini Web session. Type 'exit' or 'quit' to end."
            )
            if client.last_title:
                print(f"Resumed thread: {client.last_title}")
            elif cid:
                print(f"Resumed thread ID: {cid}")
            while True:
                try:
                    user_input = input("\nYou: ")
                    if user_input.strip().lower() in ("exit", "quit"):
                        break
                    if not user_input.strip():
                        continue
                    print("Gemini: ", end="", flush=True)
                    reply = client.generate(user_input, keep_context=True)
                    print(reply)
                except (KeyboardInterrupt, EOFError):
                    print("\nGoodbye!")
                    break

    def conversations(self, model: str = "flash") -> None:
        """List recent conversation history.

        Args:
            model: Select Gemini model ("flash", "flash-lite", "pro").
        """
        with gemini_web.GeminiWebClient(model=model) as client:
            history = client.list_conversations()
        _emit(history)

    def resume(
        self,
        cid: str,
        prompt: str | None = None,
        *,
        rid: str | None = None,
        rcid: str | None = None,
        thread: bool = False,
        repl: bool = False,
        model: str = "flash",
    ) -> None:
        """Resume a specific conversation by ID.

        Args:
            cid: Conversation ID starting with ``c_``.
            prompt: Optional first query to run in this resumed session.
            rid: Optional response ID to resume.
            rcid: Optional candidate response ID to resume.
            thread: When True, return JSON formatted metadata thread details.
            repl: Force starting interactive REPL session even if prompt is provided.
            model: Select Gemini model ("flash", "flash-lite", "pro").
        """
        self(
            prompt=prompt,
            cid=cid,
            rid=rid,
            rcid=rcid,
            thread=thread,
            repl=repl,
            model=model,
        )

    def delete(self, cid: str, model: str = "flash") -> None:
        """Delete a conversation by ID.

        Args:
            cid: Conversation ID starting with ``c_``.
            model: Select Gemini model ("flash", "flash-lite", "pro").
        """
        with gemini_web.GeminiWebClient(model=model) as client:
            client.delete_conversation(cid)
        _emit({"deleted": cid})

    def scrape(
        self, out: str | None = None, json_out: str | None = None
    ) -> None:
        """Scrape and analyze features, CSS classes, and JS functions from the Gemini Web App.

        Args:
            out: Optional path to save the Markdown report.
            json_out: Optional path to save the raw JSON data.
        """
        from pathlib import Path

        from aphrody.gemini_scraper import GeminiScraper

        scraper = GeminiScraper()
        data = scraper.scrape()

        # Save JSON if requested
        if json_out:
            p = Path(json_out)
            p.parent.mkdir(parents=True, exist_ok=True)
            import json

            p.write_text(
                json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8"
            )

        # Format markdown
        report_md = scraper.format_markdown_report(data)
        if out:
            p = Path(out)
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(report_md, encoding="utf-8")

        _emit(
            {
                "script_bundles_count": len(data["script_urls"]),
                "css_classes_count": len(data["css_classes"]),
                "css_variables_count": len(data["css_variables"]),
                "rpc_services_count": len(data["rpc_services"]),
                "rpc_methods_count": len(data["rpc_methods"]),
                "rpc_mappings_count": len(data["rpc_mappings"]),
                "boq_hashes_count": len(data["boq_hashes"]),
                "buttons_count": len(data["buttons"]),
                "models_found": data["models"],
                "markdown_saved_to": out,
                "json_saved_to": json_out,
            }
        )

    def auto_upgrade(self) -> None:
        """Run the autonomous scraping and deep learning loop to upgrade aphrody features."""
        import json
        import logging
        import subprocess
        import sys
        from pathlib import Path

        import aphrody
        from aphrody.gemini_scraper import GeminiScraper

        cli_logger = logging.getLogger("aphrody.cli.auto_upgrade")
        cli_logger.setLevel(logging.INFO)

        cli_logger.info(
            "Initializing Scrapy static code crawler for Gemini App..."
        )
        scraper = GeminiScraper()
        data = scraper.scrape()

        client_file = Path(aphrody.__file__).parent / "gemini_web.py"
        if not client_file.exists():
            raise FileNotFoundError(
                f"Could not locate client file: {client_file}"
            )

        original_code = client_file.read_text(encoding="utf-8")
        backup_file = client_file.with_suffix(".py.bak")
        backup_file.write_text(original_code, encoding="utf-8")

        replacements = []
        try:
            from aphrody.vertex import GeminiVertex

            cli_logger.info(
                "Invoking Gemini Vertex LLM for deep reasoning and mapping analysis..."
            )
            gv = GeminiVertex()

            prompt = f"""
You are a Senior Python & API Engineer. We scraped the latest Gemini Web App frontend bundles and extracted the following Boq action hash mappings:
{json.dumps(data["rpc_mappings"], indent=2)}

Here is the current implementation of our Gemini Web client (gemini_web.py):
{original_code}

Compare the mappings in the code with the scraped mappings. Specifically, look at the action hashes used for listing conversations (currently 'MaZiqc', mapping to 'BardFrontendService.ListConversations') and deleting conversations (currently 'GzXR5e', mapping to 'BardFrontendService.DeleteConversation').

If any of these hashes (or other hashes/constants for endpoints in the code) have changed in the scraped mappings, output a JSON structure specifying the exact text replacements required in the code.
Output format MUST be a valid JSON object with a single key 'replacements', containing a list of objects with 'file', 'target', and 'replacement' keys. The 'file' key must be "aphrody/aphrody/gemini_web.py".
If no replacements are needed, return an empty list under 'replacements'.
Do not return any explanations, markdown, or other text outside the JSON. Return only the raw JSON.
"""
            response_text = gv.generate(prompt)

            # Clean and parse JSON response
            cleaned = response_text.strip()
            if cleaned.startswith("```"):
                lines = cleaned.splitlines()
                if lines[0].startswith("```"):
                    lines = lines[1:]
                if lines[-1].startswith("```"):
                    lines = lines[:-1]
                cleaned = "\n".join(lines).strip()

            replacements_data = json.loads(cleaned)
            replacements = replacements_data.get("replacements", [])
        except Exception as e:
            cli_logger.warning(
                "LLM-based deep learning upgrade failed or was bypassed: %s", e
            )
            replacements = []

        # Fallback to programmatic check
        if not replacements:
            cli_logger.info("Running programmatic mapping fallback lookup...")
            scraped_mappings = data.get("rpc_mappings", {})
            list_hash = None
            delete_hash = None
            for h, m in scraped_mappings.items():
                if m == "BardFrontendService.ListConversations":
                    list_hash = h
                elif m == "BardFrontendService.DeleteConversation":
                    delete_hash = h

            fallback_replacements = []
            if list_hash and list_hash != "MaZiqc":
                cli_logger.info(
                    "Detected new ListConversations hash: %s", list_hash
                )
                fallback_replacements.append(
                    {
                        "file": "aphrody/aphrody/gemini_web.py",
                        "target": "MaZiqc",
                        "replacement": list_hash,
                    }
                )
            if delete_hash and delete_hash != "GzXR5e":
                cli_logger.info(
                    "Detected new DeleteConversation hash: %s", delete_hash
                )
                fallback_replacements.append(
                    {
                        "file": "aphrody/aphrody/gemini_web.py",
                        "target": "GzXR5e",
                        "replacement": delete_hash,
                    }
                )
            replacements = fallback_replacements

        if replacements:
            cli_logger.info(
                "Applying %d code replacements...", len(replacements)
            )
            new_code = original_code
            for rep in replacements:
                target = rep["target"]
                replacement = rep["replacement"]
                if target in new_code:
                    new_code = new_code.replace(target, replacement)
                    cli_logger.info(
                        "Replaced '%s' with '%s'", target, replacement
                    )
                else:
                    cli_logger.warning(
                        "Target string '%s' not found in gemini_web.py", target
                    )

            # Write the updated code
            client_file.write_text(new_code, encoding="utf-8")

            # Run validation checks
            cli_logger.info("Running code quality validations...")
            workspace_root = Path(aphrody.__file__).parent.parent

            # Ruff format & check
            fmt_res = subprocess.run(
                [sys.executable, "-m", "ruff", "format", str(client_file)],
                capture_output=True,
                check=False,
            )
            check_res = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "ruff",
                    "check",
                    "--fix",
                    str(client_file),
                ],
                capture_output=True,
                check=False,
            )

            # Pytest
            test_res = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "pytest",
                    "-v",
                    "aphrody/tests/test_gemini_web.py",
                ],
                cwd=str(workspace_root),
                capture_output=True,
                check=False,
            )

            validation_passed = (
                fmt_res.returncode == 0
                and check_res.returncode == 0
                and test_res.returncode == 0
            )

            if not validation_passed:
                cli_logger.error(
                    "Validation failed! Rolling back changes to original state."
                )
                client_file.write_text(original_code, encoding="utf-8")
                if backup_file.exists():
                    backup_file.unlink()
                raise RuntimeError(
                    "Autonomous upgrade failed validation tests. Rolled back successfully."
                )
            else:
                cli_logger.info(
                    "Validation passed successfully! Code upgraded."
                )
                if backup_file.exists():
                    backup_file.unlink()
        else:
            cli_logger.info(
                "No upgrades required. Codebase features are fully up to date."
            )
            if backup_file.exists():
                backup_file.unlink()

        _emit(
            {
                "success": True,
                "scraped_mappings_count": len(data["rpc_mappings"]),
                "replacements_applied": replacements,
                "upgraded": len(replacements) > 0,
            }
        )
