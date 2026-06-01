# SPDX-License-Identifier: Apache-2.0

"""Layout-based document parsing and chunking.

Converts documents (PDF, Word, Markdown, HTML, etc.) to markdown,
extracts structural layout elements (headings, code blocks, lists, tables, text),
and merges them into chunks respecting layout boundaries and token limits.
"""

import logging
import os
import re
import tempfile
from typing import Any

try:
    import tiktoken
except ImportError:
    tiktoken = None

try:
    from markitdown import MarkItDown
except ImportError:
    MarkItDown = None

logger = logging.getLogger(__name__)


class LayoutElement:
    """Represents a structural element in the document layout."""

    def __init__(self, type_: str, content: str, level: int = 0):
        self.type = type_  # "heading", "text", "list", "code_block", "table", "blockquote"
        self.content = content.strip()
        self.level = level  # for headings (1-6) or indent level

    def __repr__(self) -> str:
        return f"LayoutElement(type={self.type}, level={self.level}, content={self.content[:30]}...)"


class LayoutChunker:
    """Layout-aware chunker that parses document structures and merges them logically."""

    def __init__(
        self,
        chunk_token_num: int = 512,
        overlap_token_num: int = 64,
        encoding_name: str = "cl100k_base",
    ):
        self.chunk_token_num = chunk_token_num
        self.overlap_token_num = overlap_token_num
        self.encoding_name = encoding_name

        if tiktoken:
            try:
                self._encoder = tiktoken.get_encoding(encoding_name)
            except Exception:
                self._encoder = None
        else:
            self._encoder = None

    def _count_tokens(self, text: str) -> int:
        """Count tokens in text, fallback to approximate word count if tiktoken is missing."""
        if self._encoder:
            return len(self._encoder.encode(text))
        # fallback: 1 token ≈ 4 characters or ~0.75 words
        return len(text.split())

    def parse_document(
        self, file_path: str, binary_data: bytes | None = None
    ) -> str:
        """Convert a document to markdown layout using MarkItDown."""
        if MarkItDown is None:
            raise ImportError(
                "markitdown is required for document parsing. Install it or pass raw text directly."
            )

        md = MarkItDown()
        if binary_data is not None:
            ext = (
                "." + file_path.rsplit(".", maxsplit=1)[-1]
                if "." in file_path
                else ""
            )
            with tempfile.NamedTemporaryFile(suffix=ext, delete=False) as f:
                f.write(binary_data)
                temp_path = f.name
            try:
                result = md.convert(temp_path)
                return result.text_content
            finally:
                if os.path.exists(temp_path):
                    os.remove(temp_path)
        else:
            result = md.convert(file_path)
            return result.text_content

    def extract_elements(self, markdown_content: str) -> list[LayoutElement]:
        """Extract layout elements from markdown content."""
        lines = markdown_content.split("\n")
        elements = []
        i = 0
        n = len(lines)

        while i < n:
            line = lines[i]
            stripped = line.strip()

            if not stripped:
                i += 1
                continue

            # Heading Check (e.g. # Heading)
            heading_match = re.match(r"^(#{1,6})\s+(.*)$", line)
            if heading_match:
                level = len(heading_match.group(1))
                content = heading_match.group(2)
                elements.append(LayoutElement("heading", content, level))
                i += 1
                continue

            # Code Block Check (```python)
            if stripped.startswith("```"):
                code_lines = [line]
                i += 1
                while i < n:
                    code_lines.append(lines[i])
                    if lines[i].strip().startswith("```"):
                        i += 1
                        break
                    i += 1
                elements.append(
                    LayoutElement("code_block", "\n".join(code_lines))
                )
                continue

            # Blockquote Check (> Text)
            if stripped.startswith(">"):
                quote_lines = [stripped.lstrip(">").strip()]
                i += 1
                while i < n and lines[i].strip().startswith(">"):
                    quote_lines.append(lines[i].strip().lstrip(">").strip())
                    i += 1
                elements.append(
                    LayoutElement("blockquote", "\n".join(quote_lines))
                )
                continue

            # List Block Check (- item, * item, 1. item)
            list_match = re.match(r"^\s*([-*+]|\d+\.)\s+(.*)$", line)
            if list_match:
                list_lines = [line]
                i += 1
                while i < n:
                    sub_line = lines[i]
                    if not sub_line.strip():
                        # Peek ahead to see if list continues
                        j = i + 1
                        while j < n and not lines[j].strip():
                            j += 1
                        if j < n and (
                            re.match(r"^\s*[-*+]\s+.*$", lines[j])
                            or re.match(r"^\s*\d+\.\s+.*$", lines[j])
                        ):
                            list_lines.extend(lines[i : j + 1])
                            i = j + 1
                        else:
                            break
                    elif re.match(
                        r"^\s*([-*+]|\d+\.)\s+(.*)$", sub_line
                    ) or re.match(r"^\s+\S+.*$", sub_line):
                        list_lines.append(sub_line)
                        i += 1
                    else:
                        break
                elements.append(LayoutElement("list", "\n".join(list_lines)))
                continue

            # Table Check (lines containing |)
            if "|" in line:
                table_lines = [line]
                i += 1
                while i < n and "|" in lines[i]:
                    table_lines.append(lines[i])
                    i += 1
                table_content = "\n".join(table_lines)
                if re.search(r"\|(?:\s*[:-]+[-| :]*\s*)\|", table_content):
                    elements.append(LayoutElement("table", table_content))
                    continue
                else:
                    # Treat as standard text
                    elements.append(LayoutElement("text", table_content))
                    continue

            # Default: Paragraph text
            para_lines = [line]
            i += 1
            while i < n:
                sub_line = lines[i]
                sub_stripped = sub_line.strip()
                if not sub_stripped:
                    i += 1
                    break
                if (
                    re.match(r"^#{1,6}\s+.*$", sub_line)
                    or sub_stripped.startswith("```")
                    or sub_stripped.startswith(">")
                    or re.match(r"^\s*([-*+]|\d+\.)\s+(.*)$", sub_line)
                    or "|" in sub_line
                ):
                    break
                para_lines.append(sub_line)
                i += 1
            elements.append(LayoutElement("text", "\n".join(para_lines)))

        return elements

    def chunk_elements(
        self, elements: list[LayoutElement]
    ) -> list[dict[str, Any]]:
        """Group elements into layout-aware chunks respecting token limits and hierarchies."""
        chunks = []
        current_chunk_elements = []
        current_token_count = 0
        current_headings = []

        for elem in elements:
            elem_tokens = self._count_tokens(elem.content)

            if elem.type == "heading":
                current_headings = [
                    h for h in current_headings if h["level"] < elem.level
                ]
                current_headings.append(
                    {"text": elem.content, "level": elem.level}
                )

            if current_token_count + elem_tokens > self.chunk_token_num:
                if current_chunk_elements:
                    chunks.append(
                        self._build_chunk(
                            current_chunk_elements, current_headings
                        )
                    )
                    overlap_elements = []
                    overlap_tokens = 0
                    for prev_elem in reversed(current_chunk_elements):
                        prev_tokens = self._count_tokens(prev_elem.content)
                        if (
                            overlap_tokens + prev_tokens
                            <= self.overlap_token_num
                        ):
                            overlap_elements.insert(0, prev_elem)
                            overlap_tokens += prev_tokens
                        else:
                            break
                    current_chunk_elements = overlap_elements
                    current_token_count = overlap_tokens

            current_chunk_elements.append(elem)
            current_token_count += elem_tokens

        if current_chunk_elements:
            chunks.append(
                self._build_chunk(current_chunk_elements, current_headings)
            )

        return chunks

    def _build_chunk(
        self, elements: list[LayoutElement], headings: list[dict[str, Any]]
    ) -> dict[str, Any]:
        """Compile a list of layout elements into a final chunk dictionary."""
        content = "\n\n".join(e.content for e in elements)
        types = list(set(e.type for e in elements))

        context_prefix = ""
        if headings:
            heading_titles = [h["text"] for h in headings]
            context_prefix = " > ".join(heading_titles)

        return {
            "content": content,
            "heading_context": context_prefix,
            "layout_types": types,
            "elements_count": len(elements),
            "token_count": self._count_tokens(content),
        }

    def chunk_text(self, markdown_text: str) -> list[dict[str, Any]]:
        """Chunk a markdown string directly using layout parsing."""
        elements = self.extract_elements(markdown_text)
        return self.chunk_elements(elements)

    def chunk_document(
        self, file_path: str, binary_data: bytes | None = None
    ) -> list[dict[str, Any]]:
        """End-to-end document layout parsing and chunking."""
        markdown_text = self.parse_document(file_path, binary_data)
        elements = self.extract_elements(markdown_text)
        return self.chunk_elements(elements)
