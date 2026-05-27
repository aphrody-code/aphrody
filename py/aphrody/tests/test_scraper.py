# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Tests for the Gemini Web App scraper in :mod:`aphrody.gemini_scraper`."""

from __future__ import annotations

from aphrody.gemini_scraper import (
    GeminiScraper,
    _get_gemini_spider_class,
    _parse_contents,
)


def test_fetch_page_and_bundles(httpx_mock) -> None:
    html_content = """
    <html>
      <head>
        <link rel="preload" href="/_/BardChatUi/js/main.js" as="script">
        <link rel="stylesheet" href="style.css">
      </head>
      <body>
        <script src="https://www.gstatic.com/another.js"></script>
        <script src="//example.com/relative.js"></script>
      </body>
    </html>
    """
    httpx_mock.add_response(
        url="https://gemini.google.com/app",
        text=html_content,
    )

    scraper = GeminiScraper()
    html, urls = scraper.fetch_page_and_bundles()

    assert "main.js" in html
    assert len(urls) == 3
    assert "https://gemini.google.com/_/BardChatUi/js/main.js" in urls
    assert "https://www.gstatic.com/another.js" in urls
    assert "https://example.com/relative.js" in urls


def test_fetch_bundle(httpx_mock) -> None:
    url = "https://www.gstatic.com/another.js"
    httpx_mock.add_response(
        url=url,
        text="console.log('hello');",
    )

    scraper = GeminiScraper()
    content = scraper.fetch_bundle(url)
    assert content == "console.log('hello');"


def test_scrape(httpx_mock) -> None:
    html_content = """
    <html>
      <head>
        <style>
          :root {
            --accent-color: #fff;
          }
        </style>
      </head>
      <body>
        <button class="g-button g-active">Send message</button>
        <div role="button" class="bp-icon-button">Click here</div>
        <script src="/js/bundle.js"></script>
      </body>
    </html>
    """
    js_content = """
    const service = "assistant.lamda.BardFrontendService";
    const method = "assistant.lamda.BardFrontendService/StreamGenerate";
    const hash1 = "MaZiqc";
    const hash2 = "GzXR5e";
    const cssClass = "mat-dialog-container";
    const model = "gemini-2.0-flash";
    const flag = "is_voice_enabled";
    const flag2 = "enable_dark_mode";
    const aria = "aria-expanded";
    const role = "role='checkbox'";
    const v = "var(--primary-color)";
    new _.Hx("GzXR5e",class extends _.k{constructor(a){super(a)}},U2c,[_.Mf,!1,_.Qf,"/BardFrontendService.DeleteConversation"]);
    """

    httpx_mock.add_response(
        url="https://gemini.google.com/app",
        text=html_content,
    )
    httpx_mock.add_response(
        url="https://gemini.google.com/js/bundle.js",
        text=js_content,
    )

    scraper = GeminiScraper()
    data = scraper.scrape()

    assert len(data["script_urls"]) == 1
    assert "g-button" in data["css_classes"]
    assert "g-active" in data["css_classes"]
    assert "bp-icon-button" in data["css_classes"]
    assert "mat-dialog-container" in data["css_classes"]

    assert "--accent-color" in data["css_variables"]
    assert "--primary-color" in data["css_variables"]

    assert "assistant.lamda.BardFrontendService" in data["rpc_services"]
    assert (
        "assistant.lamda.BardFrontendService/StreamGenerate"
        in data["rpc_methods"]
    )
    assert "MaZiqc" in data["boq_hashes"]
    assert "GzXR5e" in data["boq_hashes"]
    assert "GzXR5e" in data["rpc_mappings"]
    assert (
        data["rpc_mappings"]["GzXR5e"]
        == "BardFrontendService.DeleteConversation"
    )

    assert "gemini-2.0-flash" in data["models"]
    assert "is_voice_enabled" in data["feature_flags"]
    assert "enable_dark_mode" in data["feature_flags"]
    assert "aria-expanded" in data["aria_attributes"]

    assert any(btn["text"] == "Send message" for btn in data["buttons"])
    assert any(btn["text"] == "Click here" for btn in data["buttons"])


def test_format_markdown_report() -> None:
    data = {
        "script_urls": ["https://gemini.google.com/js/bundle.js"],
        "css_classes": ["g-button", "bp-icon"],
        "css_variables": ["--accent-color", "--primary-color"],
        "rpc_services": ["assistant.lamda.BardFrontendService"],
        "rpc_methods": ["assistant.lamda.BardFrontendService/StreamGenerate"],
        "rpc_mappings": {"GzXR5e": "BardFrontendService.DeleteConversation"},
        "boq_hashes": ["MaZiqc"],
        "interactive_roles": ["button"],
        "aria_attributes": ["aria-expanded"],
        "models": ["gemini-2.0-flash"],
        "feature_flags": ["is_voice_enabled"],
        "buttons": [{"tag": "button", "text": "Send"}],
    }

    scraper = GeminiScraper()
    report = scraper.format_markdown_report(data)

    assert "# Gemini Web App Static Code Analysis Report" in report
    assert "bundle.js" in report
    assert "g-button" in report
    assert "--accent-color" in report
    assert "GzXR5e" in report
    assert "DeleteConversation" in report
    assert "BardFrontendService" in report
    assert "MaZiqc" in report
    assert "gemini-2.0-flash" in report
    assert "is_voice_enabled" in report
    assert "Send" in report


def test_scrapy_spider_callbacks() -> None:
    """Test the dynamic Scrapy Spider parser and selectors logic using offline mock responses."""
    from scrapy.http import HtmlResponse, TextResponse

    gemini_spider_cls = _get_gemini_spider_class()
    spider = gemini_spider_cls(
        user_agent="test-agent", start_url="https://gemini.google.com/app"
    )

    html = """
    <html>
      <head>
        <link rel="preload" href="/_/BardChatUi/js/main.js" as="script">
      </head>
      <body>
        <script src="https://www.gstatic.com/another.js"></script>
      </body>
    </html>
    """
    response = HtmlResponse(
        url="https://gemini.google.com/app",
        body=html.encode("utf-8"),
        encoding="utf-8",
    )

    requests = list(spider.parse(response))

    # Verify requests generated for script bundles
    assert len(requests) == 2
    assert spider.scraped_data["html"] == html
    assert (
        "https://gemini.google.com/_/BardChatUi/js/main.js"
        in spider.scraped_data["script_urls"]
    )
    assert (
        "https://www.gstatic.com/another.js"
        in spider.scraped_data["script_urls"]
    )

    # Test parse_js callback
    js_content = "console.log('test');"
    js_response = TextResponse(
        url="https://www.gstatic.com/another.js",
        body=js_content.encode("utf-8"),
        encoding="utf-8",
    )
    spider.parse_js(js_response, url="https://www.gstatic.com/another.js")
    assert (
        spider.scraped_data["js_contents"]["https://www.gstatic.com/another.js"]
        == js_content
    )


def test_parse_contents() -> None:
    """Verify parser extracts features correctly from strings."""
    html = "<style>:root { --test-var: 12px; }</style><button>Click me</button>"
    bundles = ["const flag = 'enable_something';"]
    data = _parse_contents(html, ["/js/1.js"], bundles)
    assert "--test-var" in data["css_variables"]
    assert any(btn["text"] == "Click me" for btn in data["buttons"])
    assert "enable_something" in data["feature_flags"]
