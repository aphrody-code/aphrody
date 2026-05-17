<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody Privacy Policy

## 1. The promise

aphrody collects ZERO telemetry. No usage stats, no error reports, no
analytics, no phone-home. Period. There is no background channel from
your machine to ours, ever.

## 2. What "zero" means in practice

- No `tracing` instrumentation sends data anywhere. Spans go to
  stdout/stderr for your eyes only.
- No `reqwest` calls to aphrody-controlled domains. Verify:
  `grep -r "aphrody.dev" crates/` should only show docs and CLI help
  strings, never a network destination.
- No `opentelemetry`, `jaeger`, `datadog`, or `sentry` crates in
  `[workspace.dependencies]` of the root `Cargo.toml`.
- `aphrody doctor` reads LOCAL state only — processes, network
  interfaces, disk, config. It does NOT POST anything.
- `aphrody --json` output is for YOU to pipe into `jq`; the binary does
  not transmit it.

## 3. What WE see (effectively nothing)

As maintainers of this project, the only data we ever see about you is:

- Public GitHub data: stars, forks, issues, PRs you choose to open.
- Public crates.io download counters: one aggregate integer, no
  per-user attribution.
- Public docs.rs hit counts: same aggregate-only model.
- GitHub Security Advisory submissions, only when you choose to file one.

We do NOT have, and never will have:

- Per-user activity logs.
- Per-install fingerprints or unique IDs.
- IP addresses, user agents, or build hashes of YOUR binary.

## 4. What aphrody DOES make network calls for (opt-in each time)

Each is a user-initiated subcommand, never a background task:

- `aphrody dns <domain>` — DNS query YOU asked for.
- `aphrody scrape <url>` — fetches the URL YOU specified.
- `aphrody a2a <prompt>` — connects to YOUR configured peer agent.
- `aphrody auth` — OAuth2 to YOUR Google account; token stored locally.
- `aphrody chromium sync` — reads YOUR local Chromium profile.

None transmit anything to aphrody-controlled servers. Destinations are
the ones you put on the command line.

## 5. Local data aphrody writes

- `inbox-from-<self>.jsonl` — local A2A coord log, read by a peer only
  if you configured one.
- `heartbeat-<self>.txt` — local proof-of-life timestamp.
- `path.json` and `monorepo-map.json` (from `mrx scan`) — gitignored,
  cwd-local.
- `~/.config/aphrody/` — local config and OAuth tokens.
- Cargo build artefacts under `target/` — standard Rust output.

These files NEVER leave your machine unless YOU upload them.

## 6. WASM (`aphrody-wasm`) in browsers

- The WASM module runs in YOUR browser sandbox.
- It does NOT phone home.
- It uses `getrandom` feature `js` for entropy — a Web Crypto API call
  inside your browser, no network.
- No aphrody telemetry pipeline exists for the browser to feed.

## 7. Third-party deps that COULD telemetry

We audit transitive dependencies via `cargo deny check` and `cargo vet`.
The `deny.toml` policy bans tracking-style crates. If you spot a
transitive dependency that calls home, open a Security Advisory per
[`SECURITY.md`](../SECURITY.md) and we will treat it as a hard bug.

## 8. Verify yourself

Don't trust this document — verify it:

- `grep -r "reqwest\|hyper::Client\|surf::get" crates/ | grep -v tests/`
  should only return user-invoked subcommand code paths.
- `cargo auditable info target/release/aphrody | jq '.packages[] | select(.name | test("telemetry|analytics|sentry|datadog"))'`
  should return an empty list.
- `aphrody doctor --json` output contains zero PII; pipe through
  `jq 'paths(scalars) as $p | $p'` to enumerate every key.

## 9. Changes to this policy

This file is versioned in git. Any change requires a PR that explicitly
calls out the change in [`CHANGELOG.md`](../CHANGELOG.md) under a
`### Privacy` heading. We commit to NEVER adding background telemetry
without both bumping the major version AND obtaining explicit
opt-in user consent at runtime.

## 10. Contact

Privacy questions: `community@aphrody.dev`. Security vulnerabilities:
see [`SECURITY.md`](../SECURITY.md) for the disclosure process.
