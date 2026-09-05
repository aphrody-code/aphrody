<!-- SPDX-License-Identifier: Apache-2.0 -->
# Troubleshooting Linux Environments

This troubleshooting guide resolves common build, runtime, and diagnostic issues encountered when running Aphrody on Linux hosts.

---

## 1. Cargo Build Target Errors (MSVC vs Linux)

### Issue
Running a plain `cargo build` or `cargo ci-offline` command fails with compilation errors in native modules (like `aws-lc-sys`):
```text
cc: error: unrecognized command-line option '-W0'
```

### Cause
The monorepo contains a `.cargo/config.toml` that configures `x86_64-pc-windows-msvc` as the default target. When building on Linux without specifying a target, cargo attempts to compile for MSVC using the Linux GNU compiler, which fails because the compiler does not recognize MSVC flags.

### Solution
Always specify the target triple when building or checking on Linux:
```bash
# Check the workspace
cargo check --workspace --target x86_64-unknown-linux-gnu

# Run Clippy
cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
```

---

## 2. PortAudio / PulseAudio Initialization Failures

### Issue
When running Python unit tests via `pytest`, the test runner crashes during the collection phase with:
```text
sounddevice.PortAudioError: Error initializing PortAudio: Unanticipated host error [PaErrorCode -9999]: 'PulseAudio_Initialize: Can't connect to server'
```

### Cause
The voice module (`google.antigravity.voice` and `aphrody-voice`) imports the `sounddevice` package, which immediately initializes PortAudio. If the Linux host is running in a headless environment (like a server or VPS) without a running audio server (PulseAudio / ALSA), the initialization throws a fatal error.

### Solution
Skip or ignore the voice directories and test files during test execution on headless servers:
```bash
# Run pytest ignoring voice modules
uv run pytest -m "not live_api" \
  --ignore=antigravity-sdk-python/google/antigravity/voice \
  --ignore=aphrody/tests/test_voice.py
```

---

## 3. Google Session Cookie Expiration or Redirects

### Issue
Browser scraping logs show Playwright navigating to Project Genie but immediately failing because the page redirects to Google Accounts:
```text
Redirected to Google login page. Please log in or provide a valid cookie jar.
```

### Cause
Google session cookies (specifically `__Secure-1PSID` and `__Secure-1PSIDTS`) are subject to strict IP address and location matching. If the session cookies were exported from your local machine and run on a different datacenter IP address, Google invalidates the session and redirects Chromium to the identifier sign-in page.

### Solution
- **Local Testing**: Set `APHRODY_GENIE_MOCK=1` to run in dry-run mode, bypassing the browser login step.
- **Production Deployment**: If scraping is required, use a residential proxy server that matches the geographical location from which the cookies were exported, or re-run the login steps locally using a persistent Chrome profile (`--profile-dir`).
