# Autonomous Agent Registry & Concurrency Rules

This file outlines the active agent roles, task routing, and execution bounds within the `aphrody-py` framework.

## Active Agent Roles

| Agent Role | Subsystem Target | Core Function | Whitelisted Tools |
| :--- | :--- | :--- | :--- |
| **SoulCreator** | [soul_creator.py](file:///C:/src/aphrody-py/aphrody/aphrody/soul_creator.py) | Scrapes and traverses wikis, fandoms, and media catalogs. | `httpx`, `html.parser`, `urllib` |
| **BackgroundReview** | [background_review.py](file:///C:/src/aphrody-py/aphrody/aphrody/background_review.py) | Conducts post-session self-improvement loops. | `GeminiWebClient`, local files |
| **CommandGuard** | [command_guard.py](file:///C:/src/aphrody-py/aphrody/aphrody/command_guard.py) | Enforces headless execution security and blocks unsafe shell commands. | `re`, `unicodedata` |
| **TimeoutMonitor** | [timeout_monitor.py](file:///C:/src/aphrody-py/aphrody/aphrody/timeout_monitor.py) | Aborts hanging processes and intercepts runaway infinite loops. | `subprocess`, `concurrent.futures` |

## Concurrency & Lock Protocols

1. **Local State Database**: All message logs and session details must be committed to [session_db.py](file:///C:/src/aphrody-py/aphrody/aphrody/session_db.py) using transaction blocks. 
2. **File Modifications**: Background threads performing memory or skill reviews must use atomic file edits (writing to temporary files and replacing them) to avoid corruption.
3. **Execution Timeout**: Any subprocess or crawling runner must register with the `TimeoutMonitor` using a maximum timeout of 600 seconds.
