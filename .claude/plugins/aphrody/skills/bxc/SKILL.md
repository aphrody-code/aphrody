---
name: bxc
description: Crawler and web scraping engine with advanced stealth, browser automation, and platform scrapers.
when_to_use: User asks to crawl, recon, or scrape a webpage, search Google, automate browser tasks, extract data, or interact with platforms like x.com/Grok/WorldBeyblade.
version: "1.1.0"
---

<!-- SPDX-License-Identifier: Apache-2.0 -->

# Bxc — Bun-Native Stealth Browser & Scraping Engine

`bxc` is a high-performance web browser, crawling, and scraping engine. It is optimized for server environments and Google-grade stealth, fushing in-process V8 bindings with a native Rust Chromium driver.

---

## 1. Core Subcommands

### Reconnaissance & Detection
* **Page Reconnaissance (HTML to Markdown):**
  Convert any public URL to clean, LLM-optimized Markdown:
  ```bash
  aphrody bxc recon https://example.com
  ```
* **Bot-Protection & Framework Detection:**
  Detect anti-bot systems (Cloudflare, DataDome) and frontend frameworks (React, Angular, Wiz):
  ```bash
  aphrody bxc detect https://example.com --json
  ```

### Data Scraping & Web Search
* **CSS Selector Extraction:**
  Scrape text content from specific HTML elements matching CSS selectors:
  ```bash
  aphrody bxc scrape https://example.com --selector "article h2"
  ```
* **Stealth Search Integration:**
  Perform Google searches and retrieve clean, markdown-formatted results:
  ```bash
  aphrody bxc search "rust async closures"
  ```

---

## 2. Browser Automation & Stealth Server

* **Start the Stealth CDP Server:**
  Runs a Chrome DevTools Protocol (CDP) server that Playwright, Puppeteer, or Bxc client scripts can connect to. This server bypasses advanced bot detection systems:
  ```bash
  aphrody bxc serve --cdp-port 9222
  ```
* **Site Mirroring:**
  Mirror a full website (downloading HTML, CSS, JS, and asset files locally):
  ```bash
  aphrody bxc mirror https://example.com
  ```
* **Cookie Management:**
  Manage cookie jars for authenticated browser sessions:
  ```bash
  aphrody bxc cookies --domain example.com
  ```

---

## 3. Platform Scrapers & Integrations

`bxc` includes specialized modules for crawling and automating interactions with various platforms:

* **x.com (Twitter) & xAI Grok:**
  - Scrape profiles: `aphrody bxc xcom <handle>`
  - Interact with Twitter / rank feed: `aphrody bxc x`
  - Query Grok (TTS, STT, Chat): `aphrody bxc grok "your query"`
* **World Beyblade Association:**
  Scrape and automate profiles, threads, PMs, and WBO rankings:
  ```bash
  aphrody bxc worldbeyblade --rankings
  ```
* **Challonge:**
  Extract brackets, match history, and standings from tournament pages:
  ```bash
  aphrody bxc challonge https://challonge.com/tournament_id
  ```
* **FIFA Ultimate Team (FUT):**
  Scrape player stats, market prices, and trends from FUTGG/FUTBin:
  ```bash
  aphrody bxc fut --player "Messi"
  ```
* **VoirAnime:**
  Search and resolve direct streaming catalog entries:
  ```bash
  aphrody bxc voiranime search "Inazuma Eleven"
  ```

---

## 4. Crawling Workers & Actors

For large-scale or persistent crawling tasks:
* **Run Crawler Actor:**
  Execute a pre-configured scraper actor script:
  ```bash
  aphrody bxc actor run <actor_name>
  ```
* **Persistent Crawling Daemon:**
  Run the background recursive crawler worker daemon:
  ```bash
  aphrody bxc crawl-worker --daemon
  ```

---

## 5. Global Options

* `--proxy <url>`: Route all traffic through an HTTP/SOCKS5 proxy.
* `--insecure` / `-k`: Skip TLS/SSL certificate validation.
* `--json`: Format command outputs in raw JSON.
* `--timeout <ms>`: Adjust the global network timeout.
