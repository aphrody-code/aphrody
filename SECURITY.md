# Security Policy

## Reporting a vulnerability

**Do not open a public issue.** Use one of the following private channels:

1. **Preferred** — open a [GitHub Security Advisory](https://github.com/aphrody-code/.github/security/advisories/new) directly on the affected repository.
2. **Fallback** — email the maintainer alias (resolved via the noreply forwarder on the GitHub profile).

You will get an acknowledgement within **72 hours** and a remediation timeline within **7 days**.

## Supported versions

| Branch | Status |
|--------|--------|
| `main` | Security fixes accepted |
| Previous SemVer minor | Best-effort patches for the most recent release line |
| Older | Out of support |

## Scope

In scope :
- Code published under the `aphrody-code/*` namespace
- Supply-chain compromise of declared dependencies
- Credential / token leakage in repository content

Out of scope :
- Self-hosted infrastructure (VPS) — report directly to the operator
- Third-party services consumed via API
- Issues already public on the upstream of a fork

## Disclosure

Coordinated disclosure preferred. We will credit reporters in the release notes unless requested otherwise.
