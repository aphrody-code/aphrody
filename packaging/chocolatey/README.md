# aphrody — Chocolatey package

Chocolatey packaging for the `aphrody` cross-platform CLI, aimed at corporate Windows environments where Chocolatey is the only allowed package manager.

## Install (end users, when published)

```powershell
choco install aphrody
```

Supports `x86_64` and `aarch64` Windows hosts.

## Local build

```powershell
choco pack packaging/chocolatey/aphrody.nuspec
```

## Publish (maintainers only)

```powershell
choco apikey --key <REDACTED> --source https://push.chocolatey.org/
choco push aphrody.1.0.0-canary.nupkg --source https://push.chocolatey.org/
```

Note: Chocolatey moderation queue takes 2–5 business days.

Before release, replace both `PLACEHOLDER-SHA256-AT-RELEASE-TIME` values in `tools/chocolateyinstall.ps1` with real SHA-256 of the published release zips.
