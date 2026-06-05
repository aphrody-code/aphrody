---
name: bxc
description: Crawler and web scraping engine with advanced stealth and browser automation capabilities.
when_to_use: User asks to crawl, recon, or scrape a webpage, search Google, automate browser tasks, extract data, or interact with platforms like x.com/Grok/WorldBeyblade.
version: "1.0.0"
---

# Bxc — Bun-Native Stealth Browser & Scraping Engine

Bxc is a high-performance browser engine optimized for VPS and Google-grade stealth, combining in-process V8 bindings with a native Rust Chromium driver.

## Key Subcommands

1. **Reconnaissance & Scraping:**
   - `aphrody bxc recon <url>`: One-shot URL to Markdown report.
   - `aphrody bxc detect <url>`: framework, library, and bot-protection detection.
   - `aphrody bxc scrape <url> --selector <selector>`: Extract content from matching CSS selectors.
   - `aphrody bxc search <query>`: Get clean, Markdown-formatted web search results.

2. **Stealth & Browser Automation:**
   - `aphrody bxc serve`: Starts a stealth Chrome DevTools Protocol (CDP) server.
   - `aphrody bxc mirror <url>`: Mirror/download a full site locally.
   - `aphrody bxc cookies`: Manage cookie jars for authenticated sessions.

3. **Platform Automation:**
   - `aphrody bxc xcom <handle>`: Scrape X.com profile data.
   - `aphrody bxc x`: Interactive native client/auditor for X/Twitter.
   - `aphrody bxc grok`: Query xAI Grok (TTS, STT, and Chat).
