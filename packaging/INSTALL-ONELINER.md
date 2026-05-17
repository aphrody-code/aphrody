<!-- SPDX-License-Identifier: Apache-2.0 -->

# One-liner install

Linux / macOS:

```sh
curl -fsSL https://github.com/aphrody-code/aphrody/releases/latest/download/install.sh | bash
```

Installs to `~/.local/bin/aphrody`. No `sudo`.

Windows (PowerShell):

```powershell
iwr -useb https://github.com/aphrody-code/aphrody/releases/latest/download/install.ps1 | iex
```

Installs under `%LOCALAPPDATA%\aphrody\bin\` and adds it to user PATH
(idempotent). Both scripts detect OS + arch (x86_64, aarch64), verify
SHA-256, abort on mismatch, and never invoke the binary.

## Disclaimer

Piping into a shell trades audit time for speed. To inspect first:

```sh
curl -fsSL https://github.com/aphrody-code/aphrody/releases/latest/download/install.sh -o install.sh
less install.sh && bash install.sh
```
