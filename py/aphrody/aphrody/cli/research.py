# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Deep Research command for the aphrody CLI."""

from __future__ import annotations

from aphrody.cli.utils import _emit


class ResearchCommands:
    """``aphrody research <topic>`` — deep research features."""

    def __call__(
        self,
        topic: str,
        depth: int = 2,
        out: str = "research_report.md",
        dry_run: bool = False,
    ) -> None:
        """Conduct iterative search and compile a research report on a topic.

        Args:
            topic: Research subject or query.
            depth: Number of search exploration rounds.
            out: Report Markdown file path.
            dry_run: If True, generate offline fallback report without calling API.
        """
        from aphrody.research import DeepResearcher

        dr = DeepResearcher()
        saved_path = dr.conduct_research(
            topic,
            depth=depth,
            out=out,
            dry_run=dry_run,
        )

        _emit(
            {
                "action": "deep_research",
                "saved_to": str(saved_path),
                "depth": depth,
                "topic": topic,
                "dry_run": dry_run,
            }
        )
