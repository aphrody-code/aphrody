# Scrapy Crawler & Autonomous Self-Upgrading Loop

This document outlines the architecture, design, and developer usage instructions for two key components added to the `aphrody` package:
1. **Asynchronous Scrapy Static Code Crawler** (`aphrody web scrape`)
2. **Autonomous Self-Upgrading Loop** (`aphrody web auto-upgrade`)

---

## 1. Asynchronous Scrapy Crawler

The Scrapy crawler extracts layout tokens, feature flags, target models, interactive buttons, and backend Boq RPC mappings from the live Gemini Web App without requiring any browser execution or API keys.

### Architecture
- **Asynchronous Scraping**: Utilizes Scrapy and Twisted to fetch the main app shell and download all preloaded JavaScript bundles concurrently.
- **Process Isolation**: Since Twisted's reactor cannot be restarted within the same process (which breaks multiple sequential runs or test suite execution), the crawler is executed inside an isolated `multiprocessing` process using the cross-platform `"spawn"` start method. Results are communicated back to the main process via a `multiprocessing.Queue`.
- **DRY Parsing Engine**: Regex matching logic is unified under `_parse_contents`, shared between the Scrapy crawler and HTTPX fallback.
- **HTTPX Fallback**: If Scrapy is not installed, or if the process isolation environment fails, the scraper gracefully falls back to the concurrent `httpx` and `ThreadPoolExecutor` downloader.
- **Offline-First Safeguard**: Bypasses the multiprocessing crawler during pytest execution to ensure the test runner runs 100% offline, remaining fast and compatible with `httpx_mock`.

### Verification & Testing
- Unit tests are implemented in [`tests/test_scraper.py`](../aphrody/tests/test_scraper.py).
- The Scrapy Spider's HTML selectors, XPath link extraction, relative URL resolution, and JS callbacks are tested offline using mocked Scrapy `Response` objects.

---

## 2. Autonomous Self-Upgrading Loop

The autonomous self-upgrading loop periodically audits the production Gemini Web App and automatically rewrites the client codebase to support new mappings or features.

### Core Workflow
```
[Start Check]
      │
      ▼
[Scrapy Crawl live App] ──► Extract latest Boq Hx mappings
      │
      ▼
[LLM Mapping Analysis] (Gemini Vertex)
      │
      ├─► (Success) ──► Generate JSON replacement targets
      │
      └─► (Failure / Bypassed) ──► [Programmatic Fallback Lookup]
                                              │
                                              ▼
                                   Find List/Delete hash shifts
                                              │
                                              ▼
[Apply Code Modifications] ──► gemini_web.py backup & rewrite
      │
      ▼
[Validation Checks]
      │
      ├──► ruff format
      ├──► ruff check --fix
      └──► pytest -v tests/test_gemini_web.py
      │
      ├───► (All Pass) ──► [Commit / Keep Upgraded Code]
      │
      └───► (Any Fail) ──► [Atomic Rollback to Original Code]
```

### Key Components
- **LLM Reasoning**: Queries the keyless `GeminiVertex` client with the scraped mappings and the active `gemini_web.py` client code. The model analyzes the diff and outputs a structured JSON block of target replacements.
- **Programmatic Fallback**: If regional Vertex AI access is bounded or offline, the system automatically falls back to programmatic checks (matching `ListConversations` and `DeleteConversation` to their newly resolved hashes).
- **Atomic Rollback**: Validation is strictly enforced. If `ruff` formatting, lint checks, or unit tests fail on the modified codebase, the client code is restored instantly from the backup, guaranteeing codebase stability.

### Verification & Testing
- Unit tests in [`tests/test_auto_upgrade.py`](../aphrody/tests/test_auto_upgrade.py) verify the success, LLM, fallback, and validation-rollback paths.

---

## 3. CLI Usage

### Scrape Gemini App Features
```powershell
uv run aphrody web scrape --out docs/gemini-web-app-analysis.md --json-out var/data/gemini_features.json
```

### Run Autonomous Upgrade Audit
```powershell
uv run aphrody web auto-upgrade
```

### Running on a Schedule (Continuous Loop)
The upgrade command can be registered as a background scheduled task using the agent's workspace scheduler:
```python
# Scheduled to run every 5 minutes:
schedule(
    CronExpression="*/5 * * * *",
    Prompt="Check for Gemini Web App updates and run the auto-upgrade pipeline: uv run aphrody web auto-upgrade"
)
```
This enables the agent to continuously maintain and improve the code base autonomously without human intervention.
