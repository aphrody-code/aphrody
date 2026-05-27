# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Pure-Python static scraper and parser for the Gemini Web App."""

import concurrent.futures
import importlib.util
import logging
import re
from typing import Any, ClassVar

import httpx

logger = logging.getLogger(__name__)

# Desktop User-Agent to ensure we get the full desktop web app shell.
DEFAULT_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
)

GEMINI_BASE_URL = "https://gemini.google.com"
GEMINI_APP_URL = f"{GEMINI_BASE_URL}/app"

# Feature patterns
RPC_SERVICE_RE = re.compile(r'"(assistant\.lamda\.[a-zA-Z0-9_./]+)"')
RPC_METHOD_RE = re.compile(
    r'"(assistant\.lamda\.[a-zA-Z0-9_./]+/[a-zA-Z0-9_]+)"'
)
BOQ_HASH_RE = re.compile(
    r"\b([A-Z][a-zA-Z0-9]{5})\b"
)  # Common Boq batch function names like MaZiqc, GzXR5e

# Advanced AST-like mapping regex for Hx constructors linking hashes to RPCs
BOQ_RPC_MAP_RE = re.compile(
    r'Hx\("([a-zA-Z0-9_]{5,6})",\s*.*?\s*\[\s*[^\]]*"\/([a-zA-Z0-9_]+Service\.[a-zA-Z0-9_]+)"\]'
)

# CSS class patterns (including common material/layout prefixes)
CSS_CLASS_RE = re.compile(
    r"\b((?:g|bp|mat|gemini|chat|sidebar|conversation|button)-[a-zA-Z0-9_-]+)\b"
)
CSS_VAR_RE = re.compile(r"(--[a-zA-Z0-9_-]+)")

# Interactive element selectors/attributes found in code
INTERACTIVE_ROLE_RE = re.compile(
    r'role=["\'](button|menu|dialog|tab|checkbox|combobox|listbox|option)["\']'
)
ARIA_ATTR_RE = re.compile(r"\b(aria-[a-z0-9-]+)\b")

# Model names & feature references
MODEL_REF_RE = re.compile(
    r"\b(gemini-2\.[0-9]-(?:flash|pro|ultra|lite)|flash-lite|flash|pro|ultra)\b",
    re.IGNORECASE,
)
FEATURE_FLAG_RE = re.compile(
    r"\b(enable_[a-zA-Z0-9_]+|disable_[a-zA-Z0-9_]+|is_[a-zA-Z0-9_]+_enabled)\b"
)


def _has_scrapy() -> bool:
    """Check if scrapy is available in the environment without importing it."""
    return importlib.util.find_spec("scrapy") is not None


def _get_gemini_spider_class():
    """Helper to dynamically construct the GeminiSpider class to allow lazy loading and unit testing."""
    import scrapy

    class GeminiSpider(scrapy.Spider):
        name = "gemini_spider"
        start_urls: ClassVar[list[str]] = []
        scraped_data: ClassVar[dict[str, Any]] = {
            "html": "",
            "script_urls": [],
            "js_contents": {},
        }

        def __init__(
            self, user_agent: str, start_url: str, *args: Any, **kwargs: Any
        ):
            super().__init__(*args, **kwargs)
            self.user_agent = user_agent
            self.start_urls = [start_url]

        def start_requests(self):
            # Reset class-level scraped_data to ensure clean state
            self.__class__.scraped_data = {
                "html": "",
                "script_urls": [],
                "js_contents": {},
            }
            for url in self.start_urls:
                yield scrapy.Request(
                    url=url,
                    headers={"User-Agent": self.user_agent},
                    callback=self.parse,
                )

        def parse(self, response: Any):
            self.__class__.scraped_data["html"] = response.text

            # Extract script URLs
            js_urls: set[str] = set()

            # CSS Selectors
            for src in response.css("script::attr(src)").getall():
                if src:
                    js_urls.add(src)

            # XPath Selectors
            for href in response.xpath("//link[@as='script']/@href").getall():
                if href:
                    js_urls.add(href)

            # Handle fallback preloads
            for link in response.css("link"):
                rel = link.attrib.get("rel", "")
                as_attr = link.attrib.get("as", "")
                href = link.attrib.get("href", "")
                if "preload" in rel and "script" in as_attr and href:
                    js_urls.add(href)

            # Resolve relative/absolute URLs
            absolute_urls: list[str] = []
            for url in js_urls:
                absolute_urls.append(response.urljoin(url))

            self.__class__.scraped_data["script_urls"] = absolute_urls

            # Request all bundles
            for url in absolute_urls:
                yield scrapy.Request(
                    url=url,
                    headers={"User-Agent": self.user_agent},
                    callback=self.parse_js,
                    cb_kwargs={"url": url},
                )

        def parse_js(self, response: Any, url: str):
            self.__class__.scraped_data["js_contents"][url] = response.text

    return GeminiSpider


def _run_scrapy_crawler(queue: Any, user_agent: str, start_url: str):
    """Target function for multiprocessing to run Scrapy crawler process."""
    try:
        from scrapy.crawler import CrawlerProcess

        gemini_spider_cls = _get_gemini_spider_class()

        # Run CrawlerProcess
        process = CrawlerProcess(
            settings={
                "USER_AGENT": user_agent,
                "LOG_LEVEL": "ERROR",  # Keep output quiet
                "REQUEST_FINGERPRINTER_IMPLEMENTATION": "2.7",  # Silences warning
            }
        )
        process.crawl(
            gemini_spider_cls, user_agent=user_agent, start_url=start_url
        )
        process.start()  # This blocks until Twisted reactor finishes
        queue.put((True, gemini_spider_cls.scraped_data))

    except Exception as e:
        import traceback

        queue.put((False, f"{e}\n{traceback.format_exc()}"))


def _parse_contents(
    html: str,
    script_urls: list[str],
    bundles: list[str],
) -> dict[str, Any]:
    """Unified parser that extracts all features from HTML and JS contents using regex patterns."""
    # Extracted categories
    css_classes: set[str] = set()
    css_variables: set[str] = set()
    rpc_services: set[str] = set()
    rpc_methods: set[str] = set()
    rpc_mappings: dict[str, str] = {}
    boq_hashes: set[str] = set()
    interactive_roles: set[str] = set()
    aria_attributes: set[str] = set()
    models: set[str] = set()
    feature_flags: set[str] = set()
    buttons: list[dict[str, str]] = []

    # 1. Parse main HTML shell
    # Look for buttons in the HTML
    button_tags = re.findall(r"<button\b[^>]*>(.*?)</button>", html, re.DOTALL)
    for tag in button_tags:
        text = re.sub(r"<[^>]+>", "", tag).strip()
        buttons.append({"tag": "button", "text": text})

    role_buttons = re.findall(
        r'<[a-z0-9]+[^>]*role=["\']button["\'][^>]*>(.*?)</[a-z0-9]+>',
        html,
        re.DOTALL,
    )
    for tag in role_buttons:
        text = re.sub(r"<[^>]+>", "", tag).strip()
        if text:
            buttons.append({"tag": "role=button", "text": text})

    # Scan HTML for CSS classes
    for match in CSS_CLASS_RE.finditer(html):
        css_classes.add(match.group(1))

    # Scan HTML for ARIA attributes
    for match in ARIA_ATTR_RE.finditer(html):
        aria_attributes.add(match.group(1))

    # Scan HTML inline style tags for CSS variables
    style_blocks = re.findall(r"<style[^>]*>(.*?)</style>", html, re.DOTALL)
    for block in style_blocks:
        for match in CSS_VAR_RE.finditer(block):
            css_variables.add(match.group(1))

    # 2. Parse JS bundles
    for js_content in bundles:
        # Extract CSS Classes
        for match in CSS_CLASS_RE.finditer(js_content):
            css_classes.add(match.group(1))

        # Extract CSS Variables from JS code literals
        for match in CSS_VAR_RE.finditer(js_content):
            css_variables.add(match.group(1))

        # Extract RPC services and methods
        for match in RPC_SERVICE_RE.finditer(js_content):
            rpc_services.add(match.group(1))
        for match in RPC_METHOD_RE.finditer(js_content):
            rpc_methods.add(match.group(1))
        for match in BOQ_HASH_RE.finditer(js_content):
            boq_hashes.add(match.group(1))

        # Extract Boq Hx mappings
        for match in BOQ_RPC_MAP_RE.finditer(js_content):
            rpc_mappings[match.group(1)] = match.group(2)

        # Extract Interactive roles
        for match in INTERACTIVE_ROLE_RE.finditer(js_content):
            interactive_roles.add(match.group(1))

        # Extract ARIA attributes
        for match in ARIA_ATTR_RE.finditer(js_content):
            aria_attributes.add(match.group(1))

        # Extract Model references
        for match in MODEL_REF_RE.finditer(js_content):
            models.add(match.group(1).lower())

        # Extract Feature flags
        for match in FEATURE_FLAG_RE.finditer(js_content):
            feature_flags.add(match.group(1))

    # Filter out common false-positive Boq hashes (ensure uppercase followed by lowercase mix)
    filtered_boq_hashes = {
        h
        for h in boq_hashes
        if re.match(r"^[A-Z][a-z0-9]+[A-Za-z0-9]*$", h) and len(h) == 6
    }

    # Include mapped hashes in the filtered set
    for h in rpc_mappings:
        filtered_boq_hashes.add(h)

    # Deduplicate buttons
    seen_buttons = set()
    deduped_buttons = []
    for btn in buttons:
        btn_key = (btn["tag"], btn["text"])
        if btn_key not in seen_buttons:
            seen_buttons.add(btn_key)
            deduped_buttons.append(btn)

    # Build clean sorted mappings
    sorted_mappings = {k: rpc_mappings[k] for k in sorted(rpc_mappings)}

    return {
        "script_urls": script_urls,
        "css_classes": sorted(list(css_classes)),
        "css_variables": sorted(list(css_variables)),
        "rpc_services": sorted(list(rpc_services)),
        "rpc_methods": sorted(list(rpc_methods)),
        "rpc_mappings": sorted_mappings,
        "boq_hashes": sorted(list(filtered_boq_hashes)),
        "interactive_roles": sorted(list(interactive_roles)),
        "aria_attributes": sorted(list(aria_attributes)),
        "models": sorted(list(models)),
        "feature_flags": sorted(list(feature_flags)),
        "buttons": deduped_buttons,
    }


class GeminiScraper:
    """Scraper that analyzes the Gemini Web App statically without browser execution."""

    def __init__(self, user_agent: str = DEFAULT_USER_AGENT):
        self.user_agent = user_agent
        self.headers = {
            "User-Agent": self.user_agent,
            "Accept-Language": "en-US,en;q=0.9",
        }

    def fetch_page_and_bundles(self) -> tuple[str, list[str]]:
        """Fetch the main Gemini Web App page and return its HTML and script URLs."""
        logger.info("Fetching main Gemini Web App page: %s", GEMINI_APP_URL)
        with httpx.Client(timeout=30.0, follow_redirects=True) as client:
            resp = client.get(GEMINI_APP_URL, headers=self.headers)
            resp.raise_for_status()
            html = resp.text

        # Extract JS file URLs from both script tags and preloads
        js_urls: set[str] = set()
        for match in re.finditer(r'<script[^>]+src=["\']([^"\']+)["\']', html):
            js_urls.add(match.group(1))

        # Extract link preloads
        for match in re.finditer(r"<link\b[^>]*>", html):
            link_tag = match.group(0)
            if (
                'as="script"' in link_tag
                or "as='script'" in link_tag
                or "as=script" in link_tag
            ):
                href_match = re.search(r'href=["\']([^"\']+)["\']', link_tag)
                if href_match:
                    js_urls.add(href_match.group(1))

        # Convert relative URLs to absolute
        absolute_urls: list[str] = []
        for url in js_urls:
            if url.startswith("//"):
                absolute_urls.append(f"https:{url}")
            elif url.startswith("/"):
                absolute_urls.append(f"{GEMINI_BASE_URL}{url}")
            elif not url.startswith("http"):
                absolute_urls.append(f"{GEMINI_BASE_URL}/{url}")
            else:
                absolute_urls.append(url)

        return html, absolute_urls

    def fetch_bundle(self, url: str) -> str:
        """Fetch a single JS bundle's content."""
        logger.info("Fetching JS bundle: %s", url)
        try:
            with httpx.Client(timeout=30.0, follow_redirects=True) as client:
                resp = client.get(url, headers=self.headers)
                resp.raise_for_status()
                return resp.text
        except Exception as e:
            logger.error("Failed to fetch bundle %s: %s", url, e)
            return ""

    def scrape_with_httpx(self, max_workers: int = 5) -> dict[str, Any]:
        """Fetch using concurrent httpx calls."""
        html, script_urls = self.fetch_page_and_bundles()

        # Download script bundles in parallel
        bundles: list[str] = []
        with concurrent.futures.ThreadPoolExecutor(
            max_workers=max_workers
        ) as executor:
            future_to_url = {
                executor.submit(self.fetch_bundle, url): url
                for url in script_urls
            }
            for future in concurrent.futures.as_completed(future_to_url):
                content = future.result()
                if content:
                    bundles.append(content)

        return _parse_contents(html, script_urls, bundles)

    def scrape_with_scrapy(self) -> dict[str, Any]:
        """Perform scraping using Scrapy in an isolated multiprocessing context."""
        import multiprocessing

        # We must use spawn method explicitly to be safe cross-platform
        ctx = multiprocessing.get_context("spawn")
        queue = ctx.Queue()
        p = ctx.Process(
            target=_run_scrapy_crawler,
            args=(queue, self.user_agent, GEMINI_APP_URL),
        )
        p.start()

        try:
            # Wait for results with a timeout of 45 seconds
            success, result = queue.get(timeout=45.0)
        except Exception as e:
            # If it timed out or queue was closed
            p.terminate()
            p.join()
            raise RuntimeError(
                f"Scrapy crawler process timed out or failed: {e}"
            ) from e

        p.join()

        if not success:
            raise RuntimeError(f"Error inside Scrapy crawler process: {result}")

        # Parse the collected contents
        html = result["html"]
        script_urls = result["script_urls"]
        bundles = [result["js_contents"].get(url, "") for url in script_urls]

        return _parse_contents(html, script_urls, bundles)

    def scrape(self, max_workers: int = 5) -> dict[str, Any]:
        """Perform full scrape and analysis of the Gemini Web App.

        Args:
            max_workers: Maximum threads to use for downloading JS bundles if using httpx.

        Returns:
            Dictionary containing scraped features, CSS classes, buttons, RPC endpoints, and options.
        """
        # In unit testing, we bypass multiprocessing/scrapy to run offline tests via httpx_mock.
        import sys

        is_testing = "pytest" in sys.modules

        if _has_scrapy() and not is_testing:
            try:
                logger.info("Using Scrapy engine for Gemini Web App crawling.")
                return self.scrape_with_scrapy()
            except Exception as e:
                logger.error(
                    "Scrapy scraper failed, falling back to HTTPX: %s", e
                )

        logger.info("Using HTTPX engine for Gemini Web App crawling.")
        return self.scrape_with_httpx(max_workers=max_workers)

    def format_markdown_report(self, data: dict[str, Any]) -> str:
        """Format the scraped analysis into a clean markdown document."""
        lines = [
            "# Gemini Web App Static Code Analysis Report",
            "",
            "This report summarizes the features, interactive elements, CSS layout tokens, and backend RPC endpoints extracted from the Gemini Web App frontend code bundles.",
            "",
            "## 1. Scraped JavaScript Bundles",
            f"Successfully resolved and analyzed **{len(data['script_urls'])}** main script and preload bundles:",
        ]
        for url in data["script_urls"]:
            lines.append(f"- [{url.rsplit('/', 1)[-1]}]({url})")

        lines.extend(
            [
                "",
                "## 2. Interactive Buttons & Elements",
                f"Found **{len(data['buttons'])}** distinct interactive buttons in the core page HTML shell:",
            ]
        )
        for btn in data["buttons"]:
            lines.append(f"- **{btn['text']}** (tag: `{btn['tag']}`)")

        lines.extend(
            [
                "",
                "## 3. CSS Classes & Layout tokens",
                f"Found **{len(data['css_classes'])}** distinct semantic and layout-related CSS classes (filtered by common prefixes):",
            ]
        )
        for cls in data["css_classes"][:50]:  # Cap at 50 for display
            lines.append(f"- `{cls}`")
        if len(data["css_classes"]) > 50:
            lines.append(f"- *... and {len(data['css_classes']) - 50} more*")

        lines.extend(
            [
                "",
                "## 4. CSS Custom Variables & Design Tokens",
                f"Found **{len(data['css_variables'])}** custom properties (design variables) defining colors, spacing, and typography:",
            ]
        )
        for var in data["css_variables"][:100]:  # Cap at 100 for display
            lines.append(f"- `{var}`")
        if len(data["css_variables"]) > 100:
            lines.append(f"- *... and {len(data['css_variables']) - 100} more*")

        lines.extend(
            [
                "",
                "## 5. JS Functions & Boq RPC Endpoints",
                "### Boq Action Hash to RPC Method Mappings",
                "Mappings used by the `batchexecute` protocol to execute remote backend actions:",
            ]
        )
        for h, m in data["rpc_mappings"].items():
            lines.append(f"- **{h}** -> `{m}`")

        lines.extend(
            [
                "",
                "### RPC Services",
            ]
        )
        for svc in data["rpc_services"]:
            lines.append(f"- `{svc}`")

        lines.append("\n### RPC Methods")
        for method in data["rpc_methods"]:
            lines.append(f"- `{method}`")

        lines.append(
            "\n### Known Boq batchexecute hashes (e.g. conversation/UI state handlers)"
        )
        for h in data["boq_hashes"][:50]:
            lines.append(f"- `{h}`")
        if len(data["boq_hashes"]) > 50:
            lines.append(f"- *... and {len(data['boq_hashes']) - 50} more*")

        lines.extend(
            [
                "",
                "## 6. Model Identifiers & Features",
                "### Target Models Reference",
            ]
        )
        for model in data["models"]:
            lines.append(f"- `{model}`")

        lines.append("\n### Feature Flags")
        for flag in data["feature_flags"][:50]:
            lines.append(f"- `{flag}`")
        if len(data["feature_flags"]) > 50:
            lines.append(f"- *... and {len(data['feature_flags']) - 50} more*")

        lines.extend(
            [
                "",
                "## 7. Accessibility & ARIA layout",
                "### Interactive Roles",
            ]
        )
        for role in data["interactive_roles"]:
            lines.append(f"- `{role}`")

        lines.append("\n### ARIA Attributes")
        for attr in data["aria_attributes"]:
            lines.append(f"- `{attr}`")

        return "\n".join(lines)
