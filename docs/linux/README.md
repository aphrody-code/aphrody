<!-- SPDX-License-Identifier: Apache-2.0 -->
# Linux Support in Aphrody

Linux (specifically **Ubuntu 26.04 LTS amd64/arm64**) is the **Target Cible #1** for the `aphrody` monorepo. This document outlines the build requirements, system dependencies, execution environment, and platform conventions.

---

## 1. System Requirements & Dependencies

To compile `aphrody` and run its companion tools (including scrapers, voice loops, and headless 3D generators) on Linux, install the following native dependencies:

```bash
# Core build tools, TLS libraries, and audio subsystems
sudo apt update && sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  curl \
  libasound2-dev \
  libpulse-dev \
  ffmpeg \
  ninja-build \
  cmake
```

### Headless Web & 3D Render Prerequisites
If you are running the browser automation scraper (`ProjectGenieScraper`) or 3D scene consolidator on a headless Linux server or VPS, you also need:
```bash
# Virtual Framebuffer for Playwright and Chromium anti-bot bypass
sudo apt install -y xvfb chromium-browser

# Blender (for 3D scene consolidation)
sudo apt install -y blender
```

---

## 2. Monorepo Toolchains

Aphrody is a polyglot monorepo using four pinned runtime environments:

| Language | Target Directory | Toolchain Manager | Version / Configuration |
|---|---|---|---|
| **Rust** | Root (`crates/*`) | `rust-toolchain.toml` | Nightly pinned via toolchain file |
| **Bun** (TS/JS) | Root / `apps/*` | `mise.toml` | Bun runtime & package manager |
| **Python** | `py/` | `py/pyproject.toml` | Python managed via `uv` |

---

## 3. Directory Conventions

Aphrody adheres to modern Linux path conventions:

- **Executable Deployment**: Installed to `~/.local/bin/aphrody` via the deployment script.
- **Config Home**: Configuration files are stored under `~/.config/aphrody/` (e.g. `mcp.json`).
- **Secrets Store**: Credentials and Google session cookies live in `var/secrets/` (specifically `var/secrets/google-cookies.json` and `var/secrets/antigravity-token.json`). These files must be locked to permission mode `0600` and parent directories to `0700`.
- **Temp / Generation Cache**: Temporary files, frames, and intermediate 3D meshes are written to `var/genie_temp/`.

---

## 4. Compile & Deploy

The canonical Linux compilation and deployment flow is:

```bash
# Build the primary release binary targeting Linux
cargo build --release -p aphrody --locked --target x86_64-unknown-linux-gnu

# Deploy the binary to local path (~/.local/bin/aphrody)
bash ./scripts/deploy.sh --target x86_64-unknown-linux-gnu --no-build
```

### Verification
Run system diagnostics to ensure the binary is healthy:
```bash
aphrody --version
aphrody doctor
```

---

## 5. Directory Contents

- [Headless Browser & 3D Automation](headless-automation.md) — Playwright anti-bot configuration, virtual displays (`xvfb-run`), and cookie locking.
- [Systems & OS Integration](systems-integration.md) — Libc/Nix APIs, `epoll`/`io_uring` tokio runtimes, and atomic file permissions.
- [Troubleshooting & Gotchas](troubleshooting.md) — Common compilation fixes, PulseAudio headless socket problems, and MSVC flag overrides.
