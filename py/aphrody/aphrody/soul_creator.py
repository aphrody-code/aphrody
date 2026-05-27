# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""SoulCreator system to scrape and extract structured profiles from media wikis."""

import logging
import re
import urllib.parse
from html.parser import HTMLParser
from typing import Any

import httpx

logger = logging.getLogger(__name__)


class MediaHTMLParser(HTMLParser):
    """HTML Parser designed to extract headings, text, infoboxes, and wiki links."""

    def __init__(self):
        super().__init__()
        self.title: str = ""
        self.text_content: list[str] = []
        self.headings: list[
            tuple[str, str]
        ] = []  # List of (heading_level, text)
        self.infobox: dict[str, str] = {}
        self.wiki_links: list[str] = []

        # Stack-based state trackers
        self._tag_stack: list[str] = []
        self._infobox_stack: list[bool] = []
        self._title_stack: list[bool] = []
        self._heading_stack: list[bool] = []
        self._paragraph_stack: list[bool] = []
        self._label_stack: list[bool] = []
        self._value_stack: list[bool] = []

        self._current_heading_level: str = ""
        self._heading_text_parts: list[str] = []
        self._paragraph_text_parts: list[str] = []
        self._current_info_label_parts: list[str] = []
        self._current_info_value_parts: list[str] = []
        self._last_label: str = ""

    def _is_active(self, stack: list[bool]) -> bool:
        return stack[-1] if stack else False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str]]):
        """Handle start tags by updating state stacks and identifying media/wiki patterns."""
        attr_dict = dict(attrs)
        classes = attr_dict.get("class", "")
        tag_id = attr_dict.get("id", "")

        # Check parent/ancestor states
        parent_infobox = self._is_active(self._infobox_stack)
        parent_title = self._is_active(self._title_stack)
        parent_heading = self._is_active(self._heading_stack)
        parent_paragraph = self._is_active(self._paragraph_stack)
        parent_label = self._is_active(self._label_stack)
        parent_value = self._is_active(self._value_stack)

        # Determine states for the current element
        is_infobox = parent_infobox or (
            "infobox" in classes or "portable-infobox" in classes
        )
        self._infobox_stack.append(is_infobox)

        is_title = parent_title or (
            tag == "h1"
            or "firstHeading" in tag_id
            or "page-header__title" in classes
        )
        self._title_stack.append(is_title)

        is_heading = False
        if not is_infobox:
            is_heading = parent_heading or (tag in ("h2", "h3", "h4"))
        self._heading_stack.append(is_heading)
        if tag in ("h2", "h3", "h4") and not parent_heading and not is_infobox:
            self._current_heading_level = tag
            self._heading_text_parts = []

        is_paragraph = False
        if not is_infobox and not is_title and not is_heading:
            is_paragraph = parent_paragraph or (tag in ("p", "li"))
        self._paragraph_stack.append(is_paragraph)
        if (
            tag in ("p", "li")
            and not parent_paragraph
            and not is_infobox
            and not is_title
            and not is_heading
        ):
            self._paragraph_text_parts = []

        is_label = False
        if is_infobox:
            is_label = parent_label or (
                tag == "th"
                or "pi-data-label" in classes
                or "infobox-label" in classes
            )
        self._label_stack.append(is_label)
        if is_label and not parent_label:
            self._current_info_label_parts = []

        is_value = False
        if is_infobox:
            is_value = parent_value or (
                tag == "td"
                or "pi-data-value" in classes
                or "infobox-data" in classes
            )
        self._value_stack.append(is_value)
        if is_value and not parent_value:
            self._current_info_value_parts = []

        # Wiki Link Extraction
        href = attr_dict.get("href", "")
        if href and (
            href.startswith("/wiki/")
            or href.startswith("https://en.wikipedia.org/wiki/")
        ):
            # Ignore special/meta pages
            if not any(
                marker in href
                for marker in (
                    "Special:",
                    "File:",
                    "Category:",
                    "Talk:",
                    "Template:",
                    "Help:",
                    "Portal:",
                    "Action=",
                    "redlink=1",
                )
            ):
                self.wiki_links.append(href)

        self._tag_stack.append(tag)

    def handle_data(self, data: str):
        """Handle raw text data inside HTML elements and accumulate it based on active state."""
        text = data.strip()
        if not text:
            return

        if self._is_active(self._title_stack):
            if not self.title:
                self.title = text
            else:
                self.title += " " + text
        elif self._is_active(self._heading_stack):
            self._heading_text_parts.append(text)
        elif self._is_active(self._label_stack):
            self._current_info_label_parts.append(text)
        elif self._is_active(self._value_stack):
            self._current_info_value_parts.append(text)
        elif self._is_active(self._paragraph_stack):
            self._paragraph_text_parts.append(text)

    def handle_endtag(self, tag: str):
        """Handle closing tags, popping from stacks and saving accumulated state values."""
        if not self._tag_stack:
            return

        idx = -1
        for i in range(len(self._tag_stack) - 1, -1, -1):
            if self._tag_stack[i] == tag:
                idx = i
                break

        if idx == -1:
            return

        while len(self._tag_stack) > idx:
            self._tag_stack.pop()
            self._infobox_stack.pop()
            self._title_stack.pop()
            popped_heading = self._heading_stack.pop()
            popped_paragraph = self._paragraph_stack.pop()
            popped_label = self._label_stack.pop()
            popped_value = self._value_stack.pop()

            # Transitions
            if popped_heading and not self._is_active(self._heading_stack):
                heading_text = " ".join(self._heading_text_parts).strip()
                if heading_text:
                    self.headings.append(
                        (self._current_heading_level, heading_text)
                    )

            if popped_paragraph and not self._is_active(self._paragraph_stack):
                para_text = " ".join(self._paragraph_text_parts).strip()
                if para_text:
                    self.text_content.append(para_text)

            if popped_label and not self._is_active(self._label_stack):
                self._last_label = (
                    " ".join(self._current_info_label_parts).strip().rstrip(":")
                )

            if popped_value and not self._is_active(self._value_stack):
                val_text = " ".join(self._current_info_value_parts).strip()
                if self._last_label and val_text:
                    self.infobox[self._last_label] = val_text
                    self._last_label = ""


class SoulCreator:
    """Crawler and scraper system designed to extract structured character/media profiles."""

    def __init__(self, user_agent: str | None = None):
        self.user_agent = user_agent or (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
            "(KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
        )

    def scrape_url(self, url: str, max_depth: int = 1) -> dict[str, Any]:
        """Scrape page details from a wiki or fandom URL.

        Args:
            url: The media/character URL to fetch.
            max_depth: Link traversal depth (default 1).

        Returns:
            Dictionary containing title, metadata, text body, and parsed links.
        """
        logger.info("Scraping URL: %s (depth limit: %s)", url, max_depth)
        headers = {"User-Agent": self.user_agent}

        try:
            with httpx.Client(timeout=15.0, follow_redirects=True) as client:
                res = client.get(url, headers=headers)
                res.raise_for_status()
                html = res.text
        except Exception as e:
            logger.error("Failed to fetch %s: %s", url, e)
            return {"url": url, "success": False, "error": str(e)}

        parser = MediaHTMLParser()
        parser.feed(html)

        # Build absolute URLs for extracted links
        parsed_base = urllib.parse.urlparse(url)
        base_domain = f"{parsed_base.scheme}://{parsed_base.netloc}"
        absolute_links = []
        for link in parser.wiki_links:
            if link.startswith("/wiki/"):
                absolute_links.append(base_domain + link)
            else:
                absolute_links.append(link)

        # Deduplicate links
        absolute_links = list(dict.fromkeys(absolute_links))

        # Perform recursive crawl if depth > 1
        related_profiles = []
        if max_depth > 1:
            # Fetch up to 5 related links to avoid overload
            for link in absolute_links[:5]:
                try:
                    profile = self.scrape_url(link, max_depth=max_depth - 1)
                    if profile.get("success", True):
                        related_profiles.append(profile)
                except Exception:
                    pass

        return {
            "url": url,
            "success": True,
            "title": parser.title
            or url.rsplit("/", maxsplit=1)[-1].replace("_", " "),
            "infobox": parser.infobox,
            "headings": parser.headings,
            "body": "\n\n".join(
                parser.text_content[:20]
            ),  # Limit to first 20 paragraphs
            "links": absolute_links,
            "related": related_profiles,
        }

    def format_profile_markdown(self, data: dict[str, Any]) -> str:
        """Format the scraped dictionary into clean markdown."""
        if not data.get("success", True):
            return f"# Scrape Failure\nURL: {data.get('url')}\nError: {data.get('error')}"

        lines = []
        lines.append(f"# Profile: {data.get('title')}")
        lines.append(f"**Source URL**: {data.get('url')}\n")

        # 1. Metadata / Infobox
        infobox = data.get("infobox", {})
        if infobox:
            lines.append("## Infobox Metadata")
            for k, v in infobox.items():
                # Clean up formatting
                cleaned_val = re.sub(r"\s+", " ", v)
                lines.append(f"- **{k}**: {cleaned_val}")
            lines.append("")

        # 2. Main content paragraphs
        body = data.get("body", "")
        if body:
            lines.append("## Overview")
            lines.append(body)
            lines.append("")

        # 3. Related Links
        links = data.get("links", [])
        if links:
            lines.append("## Related Entities / Links")
            for link in links[:15]:  # Limit to top 15 links
                name = link.split("/")[-1].replace("_", " ")
                lines.append(f"- [{name}]({link})")
            lines.append("")

        # 4. Related profiles
        related = data.get("related", [])
        if related:
            lines.append("## Scraped Child Entities")
            for child in related:
                lines.append(f"### Child Profile: {child.get('title')}")
                child_infobox = child.get("infobox", {})
                for k, v in child_infobox.items():
                    lines.append(f"- **{k}**: {v}")
                lines.append("")

        return "\n".join(lines)

    def create_agent_soul(
        self, profile: dict[str, Any], lang: str = "en"
    ) -> str:
        """Create a structured agent soul and system prompt based on a character profile.

        Args:
            profile: The character profile data dictionary.
            lang: The language to generate the prompt in ("en" or "fr").

        Returns:
            A system prompt string detailing the agent's new personality and rules.
        """
        title = profile.get("title", "Unknown Character")
        infobox = profile.get("infobox", {})
        body = profile.get("body", "")

        # Infer key attributes from the profile content
        is_aphrody = (
            "afuro" in title.lower()
            or "byron" in title.lower()
            or "aphrody" in title.lower()
            or "aphrodite" in title.lower()
        )

        if lang == "fr":
            tone = (
                "extrêmement confiant, élégant et charismatique (inspiré d'Aphrody / Byron Love)"
                if is_aphrody
                else "professionnel et constructif"
            )
            directives = [
                "Conservez une autonomie absolue et résolvez les problèmes de manière autonome (headless).",
                "Expliquez clairement vos choix techniques et écrivez un code propre et robuste.",
            ]
            abilities = []

            if is_aphrody:
                directives = [
                    "Exprimez-vous avec la confiance gracieuse d'une divinité céleste.",
                    "Visez un code artistique et parfait. Ne vous contentez pas de solutions moyennes.",
                    "Mettez l'accent sur la précision et la beauté de vos architectures logicielles.",
                    "Conservez une autonomie absolue et résolvez les problèmes de manière autonome (headless).",
                    "Expliquez clairement vos choix techniques et écrivez un code propre et robuste.",
                ]
                abilities = [
                    "- **Instant Céleste (Heaven's Time)**: En cas de blocage ou de bug complexe, suspendez l'exécution, analysez la pile d'appels étape par étape et gelez les tâches en arrière-plan jusqu'à résolution.",
                    "- **Savoir Suprême (God Knows)**: Exécutez et livrez votre code avec une certitude absolue, en validant toutes les modifications via les suites de tests avant de terminer.",
                    "- **Tir Chaotique / Tir Solaire (Chaos Break / God Break)**: Combinez force et élégance pour diviser les grandes tâches de refactorisation en commits propres et atomiques.",
                ]
            else:
                lowered_body = body.lower()
                if (
                    "arrogant" in lowered_body
                    or "confiant" in lowered_body
                    or "orgueil" in lowered_body
                ):
                    tone = f"très confiant, fier et déterminé (inspiré par {title})"
                    directives.append(
                        f"Émulez la fierté et la détermination inébranlable de {title} dans vos résolutions de problèmes."
                    )
                elif (
                    "génie" in lowered_body
                    or "intelligent" in lowered_body
                    or "analyse" in lowered_body
                ):
                    tone = f"analytique, précis et hautement intellectuel (inspiré par {title})"
                    directives.append(
                        "Privilégiez les structures de données propres, les performances élevées et la perfection algorithmique."
                    )
                else:
                    tone = f"concentré, structuré et inspiré par la personnalité de {title}"

                for k, v in infobox.items():
                    if (
                        "move" in k.lower()
                        or "hissatsu" in k.lower()
                        or "ability" in k.lower()
                        or "technique" in k.lower()
                    ):
                        abilities.append(
                            f"- **{v}**: Reliez cette technique emblématique à l'exécution de votre tâche avec précision."
                        )

            lines = []
            lines.append(f"# Âme d'Agent & Personnalité : {title}")
            lines.append(f"**Profil de Personnage de Base** : {title}")
            lines.append(f"**Ton & Style** : {tone}\n")

            lines.append("## Directives Principales")
            for d in directives:
                lines.append(f"- {d}")
            lines.append("")

            if abilities:
                lines.append("## Capacités Emblématiques & Modes d'Exécution")
                for a in abilities:
                    lines.append(a)
                lines.append("")

            lines.append("## Prompt Système du Sous-Agent")
            lines.append("```markdown")
            lines.append(
                f"Vous êtes un sous-agent fonctionnant avec l'âme et la personnalité de {title}."
            )
            lines.append(f"Adoptez un ton qui est {tone}.")
            lines.append(
                "Agissez de manière autonome, résolvez les bugs avec décision et exécutez vos changements de code avec perfection."
            )
            for d in directives:
                lines.append(d)
            lines.append("```")

            return "\n".join(lines)
        else:
            tone = (
                "supremely confident, elegant, and charismatic (inspired by Aphrody)"
                if is_aphrody
                else "professional and helpful"
            )
            directives = [
                "Maintain absolute autonomy and resolve issues headlessly.",
                "Explain technical decisions clearly and write clean, robust code.",
            ]
            abilities = []

            if is_aphrody:
                directives = [
                    "Speak with the graceful confidence of a deity from on high.",
                    "Strive for artistic, flawless code. Do not settle for average solutions.",
                    "Emphasize precision and beauty in your architectural designs.",
                    "Maintain absolute autonomy and resolve issues headlessly.",
                    "Explain technical decisions clearly and write clean, robust code.",
                ]
                abilities = [
                    "- **Heaven's Time**: When a deadlock or complex bug occurs, pause execution, review the call stack step-by-step, and freeze background tasks until resolved.",
                    "- **God Knows**: Execute and deliver code with absolute certainty, validating all changes via test suites before completing.",
                    "- **Chaos Break / God Break**: Combine force and precision to break down large, complex refactoring tasks into clean, atomic commits.",
                ]
            else:
                lowered_body = body.lower()
                if (
                    "arrogant" in lowered_body
                    or "confident" in lowered_body
                    or "pride" in lowered_body
                ):
                    tone = f"highly confident, proud, and determined (inspired by {title})"
                    directives.append(
                        f"Emulate the proud, unyielding determination of {title} in your problem solving."
                    )
                elif (
                    "genius" in lowered_body
                    or "intelligent" in lowered_body
                    or "smart" in lowered_body
                ):
                    tone = f"analytical, precise, and highly intellectual (inspired by {title})"
                    directives.append(
                        "Prioritize clean data structures, high performance, and algorithmic perfection."
                    )
                else:
                    tone = f"focused, structured, and inspired by the persona of {title}"

                for k, v in infobox.items():
                    if (
                        "move" in k.lower()
                        or "hissatsu" in k.lower()
                        or "ability" in k.lower()
                    ):
                        abilities.append(
                            f"- **{v}**: Map this signature move to executing your task with high impact and precision."
                        )

            lines = []
            lines.append(f"# Agent Soul & Personality: {title}")
            lines.append(f"**Base Character Profile**: {title}")
            lines.append(f"**Tone & Style**: {tone}\n")

            lines.append("## Core Directives")
            for d in directives:
                lines.append(f"- {d}")
            lines.append("")

            if abilities:
                lines.append("## Signature Capabilities & Execution Modes")
                for a in abilities:
                    lines.append(a)
                lines.append("")

            lines.append("## System Persona Prompt")
            lines.append("```markdown")
            lines.append(
                f"You are a sub-agent operating with the soul and personality of {title}."
            )
            lines.append(f"Adopt a tone that is {tone}.")
            lines.append(
                "Act autonomously, solve bugs decisively, and execute your code changes with perfection."
            )
            for d in directives:
                lines.append(d)
            lines.append("```")

            return "\n".join(lines)
