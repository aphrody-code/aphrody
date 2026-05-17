<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# IEVR — network / online observations

Online stack shipped with IEVR (`EOSSDK-Win64-Shipping.dll` +
`steam_api64.dll` + `libcurl.dll`). Defines what aphrody observes from
disk and what we refuse to RE. Read with
[`eac-considerations.md`](eac-considerations.md), `legal-checklist.md`
(pending), [`save-format-notes.md`](save-format-notes.md).

## 1. Scope statement

**IN SCOPE**

- Identifying which DLLs handle networking, by passive disk inspection.
- Documenting which Steam and EOS features are used, derived from string
  analysis of shipped binaries.
- Cloud save format, via offline analysis of locally cached saves.

**OUT OF SCOPE — never performed under aphrody**

- Live network traffic capture for cheat development.
- Multiplayer protocol RE for cheating or botting.
- Bypass of EOS authentication.
- Patching of network handshake or response payloads.
- Building a private server or matchmaking proxy.
- Exploit testing against the game's online services.

These restrictions track `legal-checklist.md` §2 (Red activities) and
[`eac-considerations.md`](eac-considerations.md) §8.

## 2. Network-related binaries observed

- **EOSSDK-Win64-Shipping.dll** (~19 MB) — Epic Online Services SDK.
  Auth, matchmaking, voice, leaderboards, achievements, cloud saves.
- **libcurl.dll** (~543 KB) — HTTP client. Likely telemetry and patch
  checks.
- **steam_api64.dll** (~320 KB) — Steamworks SDK. Friends, presence,
  achievements, cloud saves.
- **sdkencryptedappticket64.dll** (~1 MB) — Steam encrypted app ticket
  validation.

## 3. EOS features used — hypothesis to verify in P2

Likely candidates from common EOS deployments:

- **EOS_Auth** — player login (anonymous or Epic account).
- **EOS_Achievements** — achievement sync.
- **EOS_Leaderboards** — ranked leaderboards.
- **EOS_Sessions** — matchmaking for ranked battles.
- **EOS_P2P** — peer-to-peer match connections.
- **EOS_Stats** — gameplay metrics.

Verification, static only: `dumpbin /exports nie.exe | grep -i EOS_`,
cross-checked with `strings` on the SDK DLL.

## 4. Steamworks features used

- **steam_input_for_\*.vdf** — controller mapping (PS4/PS5/Switch/Xbox
  configs already inventoried).
- **steam_action_manifest.vdf** — Steam Input action mappings.
- **Steam Cloud** — save sync. Offline analysis of cached saves is
  permitted; live API capture is not.

## 5. Telemetry hypothesis

Presence of `libcurl.dll` suggests HTTP telemetry. Plausible endpoints
include crash reporting (Sentry-class), gameplay metrics, A/B config.
Verification stays offline: static scan of `nie.exe` for hardcoded
`https://` literals, with no attempt to contact the endpoints.

## 6. What we will NOT do

- No capture or decryption of live network packets.
- No reverse engineering of the EOS handshake.
- No modification of network responses.
- No private server or matchmaking proxy.
- No exploit testing against shipped online services.

Any contributor proposing one of the above must first amend
`legal-checklist.md` and obtain explicit written approval.

## 7. Cloud save observation — in scope

- Steam Cloud path: `<Steam>/userdata/<steamid>/<appid>/remote/`.
- Read-only inspection of synced saves is permitted.
- Schema work: [`save-format-notes.md`](save-format-notes.md).

## 8. References

- EOSSDK docs: <https://dev.epicgames.com/docs/services>
- Steamworks docs: <https://partner.steamgames.com/doc/sdk>
- Per-game Steam stats and leaderboard schemas are typically reachable
  through the public Steam Web API.

## 9. Open questions

- [ ] Does IEVR use EOS auth, Steam auth, or both?
- [ ] Are there hardcoded telemetry URLs in `nie.exe`?
- [ ] What is the Steam Cloud save schema for this title?
- [ ] Does the game function offline-only, with no EOS dependency?
