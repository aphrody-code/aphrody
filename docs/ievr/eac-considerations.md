<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — EAC anti-cheat impact on reverse engineering

Operational note for **Easy Anti-Cheat** as shipped with IEVR
(`EACLauncher.exe` + `EOSSDK-Win64-Shipping.dll`). Governs which RE
activities are safe with EAC present. See [`PLAN.md`](PLAN.md) §3 and
`legal-checklist.md` (pending).

## 1. What EAC is

**Easy Anti-Cheat** is a kernel-mode anti-cheat by Epic Games (acquired
2018). The launcher spawns the game and loads driver `EasyAntiCheat.sys`
at runtime. The process is then observed from ring 0 — userland defences
face a far wider detection surface than against a user-mode anti-cheat.

## 2. What EAC detects

- **Debuggers** — `IsDebuggerPresent`, `CheckRemoteDebuggerPresent`,
  `NtQueryInformationProcess` and native hooks.
- **Memory scanners** — external `ReadProcessMemory` against the game.
- **DLL injection** — Frida `agent.dll`, Cheat Engine helpers, generic
  loaders.
- **In-memory code modifications** — runtime patches to the game binary.
- **Bypass-tool fingerprints** — BattlEye / Vanguard bypass signatures
  pattern-match even though IEVR uses neither; mere presence trips.

## 3. What EAC does NOT block

- **Static analysis on disk** — Ghidra, IDA, radare2 reading files from
  `steamapps/common/...` never touch the process.
- **File-system access** — reading CPK / pak containers, parsing formats.
- **Passive network capture** — Wireshark recording traffic without
  injecting.
- **Disassembling `EasyAntiCheat.sys` itself** — legal-grey but not
  detected. Do not publish findings.

## 4. Singleplayer offline mode

Many EAC titles run checks only on multiplayer connect. IEVR appears to
ship EAC for ranked multiplayer specifically. Launching `nie.exe`
directly (bypassing `GameBootstrapper.exe` → `EACLauncher.exe`) **may**
boot story mode without the driver — to be tested under P1. Some
publishers refuse to launch at all when EAC fails to init; if that holds
here, dynamic analysis is off the table.

## 5. Frida + EAC interaction

Frida is process injection — **DETECTED**. Frida-attach to an
EAC-protected process risks an instant ban on any account ever used in
multiplayer. Attaching before EAC inits is theoretically possible and
practically fragile. Confine dynamic analysis to offline-mode launches
when reachable, or skip P4 entirely.

## 6. Steam offline mode caveat

Steam offline mode prevents matchmaking but does **not** prevent the EAC
driver from loading — driver load happens at game launch regardless of
Steam connectivity. Assume the driver is running until proven otherwise.

## 7. Per-activity safety table

| Activity | Safe with EAC? | Notes |
|---|---|---|
| Static analysis of `nie.exe` on disk | YES | Ghidra reads the file, not the process |
| CPK extraction from disk | YES | File-system reads only |
| `aphrody doctor` / `aphrody mrx scan` on install dir | YES | Read-only enumeration |
| Passive network capture (Wireshark) | YES | No injection, no hooks |
| Frida hook of `nie.exe` running with EAC | NO | Multiplayer ban likely |
| Cheat Engine / process scanner | NO | Detected immediately |
| Modifying `nie.exe` on disk | DANGER | EAC may detect hash mismatch at launch |
| RE of `EasyAntiCheat.sys` | LEGAL but DO NOT SHARE | Anti-tamper bypass is a DMCA grey area |

## 8. Mission alignment

We focus on **static analysis only** — safe with EAC present. We do not
pursue any EAC bypass, and do not touch the multiplayer protocol (legal +
EAC + ToS triple risk). Per `legal-checklist.md` _pending_ §2 and
[`PLAN.md`](PLAN.md) §3, EAC bypass is explicitly RED.

## 9. Operational checklist before any IEVR session

- [ ] Steam set to offline mode.
- [ ] No Frida, Cheat Engine, x64dbg, or debuggers running.
- [ ] Only static disk-side tools active (Ghidra, hex editors, viewers).
- [ ] Account used for analysis carries no multiplayer history.
- [ ] If the game refuses to launch offline, skip dynamic analysis and
      fall back to static-only.
