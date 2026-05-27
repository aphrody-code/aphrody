<!-- SPDX-License-Identifier: Apache-2.0 -->
# Release Automation and Workflow Guide

This document describes the automated release process for the `aphrody-ts` packages (`skills` and `design`) and their compiled standalone binaries.

## Automated Release Script

The script [release.ts](file:///c:/src/aphrody-ts/scripts/release.ts) automates the entire end-to-end release pipeline.

### Pipeline Steps

1. **Auth Check**: Automatically extracts and loads the GITHUB_TOKEN from `C:\Users\yohan\vps-mirror\.npmrc` if not already present in the environment variables.
2. **Version Bump**: Updates the `version` field in `packages/skills/package.json` and `apps/design/package.json` with the new target version.
3. **Compile Standalone Binaries**: Builds the optimized standalone executables `bin/skills.exe` and `bin/design.exe` using `bun build --compile`.
4. **Environment PATH update**: Automatically adds `bin/` to the user's system PATH via PowerShell if not already registered.
5. **Git Push Sub-repositories**: Commits and pushes changes in `packages/skills` to its respective repository (`aphrody-code/skills`).
6. **Git Push Root**: Commits and pushes all changes in `aphrody-ts` to its main repository (`aphrody-code/aphrody-ts`).
7. **NPM Scoped Packages Publication**: Triggers [publish-github-packages.ts](file:///c:/src/aphrody-ts/scripts/publish-github-packages.ts) to publish the `@aphrody-code/skills` and `@aphrody-code/design` packages to the GitHub Packages registry.
8. **Git Tagging**: Tags the commit with `v<version>` and pushes it.
9. **GitHub Release & Upload**: Creates a GitHub Release, attaches the compiled standalone binaries as assets, and publishes the release live.

---

## How to Execute a Release

To run a release, execute the following command from the repository root:

```sh
# 1. First, always run a dry-run to verify the pipeline steps:
bun scripts/release.ts --version 1.5.9 --dry-run

# 2. Run the actual release:
bun scripts/release.ts --version 1.5.9
```

---

## Technical Details

### Binary Compilation
The script builds the standalone executables under the `bin/` folder using:
- **skills.exe**: `bun build --compile ./packages/skills/src/cli.ts --outfile ./bin/skills.exe`
- **design.exe**: `bun build --compile ./apps/design/src/server.ts --outfile ./bin/design.exe`

### PATH Environment Variable Registration
The script executes a PowerShell instruction to permanently register the binary path in the Current User environment registry key:
```powershell
[System.Environment]::SetEnvironmentVariable("PATH", $currentPath + ";" + $binPath, "User")
```

### GitHub Packages Scope Resolution
GitHub Packages registry requires NPM packages to be published under the org namespace matching the repository owner (`@aphrody-code`). The publishing script rewrites `package.json` dynamically at publish-time and automatically cleans it up afterwards so local source directories remain clean.
