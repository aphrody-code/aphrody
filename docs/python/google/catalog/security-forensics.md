# Google · Security & Forensics

Google's security and forensics Python repositories cover the full defensive stack: scalable fuzzing infrastructure, remote live-forensics agents, collaborative timeline analysis, network security testing, cryptographic artifact auditing, ACL generation, USB attack prevention, and CTF challenge archives.

> Part of [`docs/python/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 29 repos (20 active / 9 archived).

---

## Fuzzing

### [clusterfuzz](https://github.com/google/clusterfuzz)
**★ 5563 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `fuzzing` `security` `stability` `vulnerabilities`

ClusterFuzz is the scalable fuzzing backend that Google uses to fuzz all its own products and that powers OSS-Fuzz. It supports libFuzzer, AFL, AFL++, and Honggfuzz with ensemble fuzzing and per-engine strategies, runs on clusters from a single machine up to 100,000 VMs, deduplicates crashes automatically, and files/triages/closes bugs in Monorail, Jira, and other trackers. The platform provides a web UI, bisection-based regression finding, testcase minimization, and per-fuzzer performance statistics.

### [oss-fuzz-gen](https://github.com/google/oss-fuzz-gen)
**★ 1398 · `active` · pushed 2026-03 · Apache-2.0**
Topics: `ai` `fuzzing` `llm` `security`

oss-fuzz-gen is a framework that uses Large Language Models (Vertex AI Gemini, OpenAI GPT-4o, and others) to automatically generate and evaluate fuzz targets for C/C++, Java, and Python projects, benchmarking them against OSS-Fuzz. Generated targets are scored on four metrics: compilability, runtime crashes, line coverage, and coverage delta versus existing human-written fuzz targets. The framework orchestrates generation, compilation, execution, and report production end-to-end.

### [fuzzbench](https://github.com/google/fuzzbench)
**★ 1195 · `active` · pushed 2026-01 · Apache-2.0**
Topics: `benchmark-framework` `benchmarking` `evaluation` `fuzzing` `security`

FuzzBench is a free service that evaluates fuzzers against a wide set of real-world OSS-Fuzz projects at Google scale, producing reproducible reports with statistical tests and coverage graphs. It provides an easy integration API for new fuzzers, uses OSS-Fuzz projects as benchmarks, and publishes public comparison reports to help the research community adopt and validate fuzzing advances.

### [atheris](https://github.com/google/atheris)
**★ 1624 · `active` · pushed 2025-11 · Apache-2.0**

Atheris is a coverage-guided Python fuzzer built on libFuzzer. It fuzzes pure-Python code and native CPython extensions (C/C++) in the same process, collecting branch coverage feedback from both layers via instrumentation. It integrates with AddressSanitizer and UndefinedBehaviorSanitizer for native extension bugs. Supports Linux and macOS, Python 3.11–3.13. Install via `pip install atheris`; native extension fuzzing requires building against a matching Clang/libFuzzer version.

### [oss-fuzz-vulns](https://github.com/google/oss-fuzz-vulns)
**★ 178 · `active` · pushed 2026-05 · CC-BY-4.0**

A data repository of OSS-Fuzz-discovered vulnerabilities published to the Open Source Vulnerabilities (OSV) database at `osv.dev`. Each entry is a structured YAML/JSON record linking a CVE or OSV identifier to the affected package, version range, and fix commit. Consumed programmatically by vulnerability scanners and dependency checkers.

---

## Forensics & Incident Response (DFIR)

### [grr](https://github.com/google/grr)
**★ 5065 · `active` · pushed 2026-05 · Apache-2.0**

GRR Rapid Response is an incident response framework for remote live forensics. A Python agent (client) is deployed to target endpoints; a Python server manages the fleet, schedules hunts, and collects forensic artifacts at scale. GRR can enumerate processes, open files, registry keys, and network connections; pull specific files or memory regions; and run YARA scans — all remotely without requiring a shell. It targets enterprise environments with hundreds of thousands of endpoints.

### [timesketch](https://github.com/google/timesketch)
**★ 3335 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `analysis` `dfir` `forensics` `security` `timeline`

Timesketch is a collaborative forensic timeline analysis tool. Analysts load events from Plaso, CSV, or JSONL sources into "sketches", then annotate, tag, and comment on events together in real time. It integrates with OpenSearch for fast full-text and field-level search, and ships a Python API client (`timesketch_api_client`) for programmatic interaction. The web UI renders timelines, context views, and graph-based relationship exploration.

### [turbinia](https://github.com/google/turbinia)
**★ 790 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `cloud` `dfir` `forensics` `security` `security-automation`

Turbinia automates the deployment and execution of distributed forensic workloads in the cloud. A server schedules tasks (Plaso log2timeline, strings, TSK, etc.) to workers that process evidence in parallel, feeding new artifacts back into the pipeline. Note: the project has entered maintenance mode; the successor project is OpenRelik. Existing deployments should plan a migration path.

### [dfiq](https://github.com/google/dfiq)
**★ 309 · `active` · pushed 2026-03 · Apache-2.0**

DFIQ (Digital Forensics Investigative Questions) is a YAML-based catalog of investigative questions, scenarios, and the step-by-step approaches for answering them during digital forensic investigations. It is tool-agnostic and designed to drive consistent, explainable investigations. The catalog is published at `dfiq.org` and is under active review for alignment with DFIQ specification version 1.1.

### [cloud-forensics-utils](https://github.com/google/cloud-forensics-utils)
**★ 503 · `active` · pushed 2026-05 · Apache-2.0**

A Python library for performing DFIR analysis on Google Cloud Platform and AWS. It provides APIs to create forensic disk snapshots and copies, mount evidence volumes, query Cloud Logging/CloudTrail, and retrieve instance metadata — all without requiring direct access to the compromised system. Used programmatically and as a dependency by Turbinia.

### [rekall](https://github.com/google/rekall)
**★ 2000 · `archived` · pushed 2020-10 · GPL-2.0**

Rekall was a comprehensive memory forensics framework supporting Windows, Linux, and macOS memory images. It evolved from Volatility and introduced a profile-based address-space abstraction that isolated memory-structure knowledge from analysis logic. The project is archived; Volatility 3 is the maintained successor for memory forensics.

### [docker-explorer](https://github.com/google/docker-explorer)
**★ 553 · `archived` · pushed 2024-10 · Apache-2.0**
Topics: `docker` `forensics`

docker-explorer helps forensicate offline Docker acquisitions by parsing Docker's storage-driver layer structures (AUFS, overlay2) to reconstruct container filesystems from disk images without a running Docker daemon. It lists containers and images found in an acquired storage root and mounts individual layers for artifact extraction.

### [GiftStick](https://github.com/google/GiftStick)
**★ 145 · `active` · pushed 2026-03 · Apache-2.0**

GiftStick provides a one-click workflow for pushing forensic evidence (disk images, memory dumps) from a live Linux system to a cloud storage bucket (Google Cloud Storage or AWS S3). A bootable USB image runs acquisition scripts that hash and upload evidence with minimal analyst interaction, suitable for first-responder scenarios.

### [amt-forensics](https://github.com/google/amt-forensics)
**★ 47 · `archived` · pushed 2021-10 · Apache-2.0**

A Python tool that retrieves Intel AMT (Active Management Technology) audit logs from a Linux machine without knowing the AMT admin password, by exploiting the unauthenticated `StartOptIn` audit-log API. Useful for investigating AMT activity on endpoints as part of an incident response.

### [picatrix](https://github.com/google/picatrix)
**★ 54 · `active` · pushed 2025-03 · Apache-2.0**

Picatrix is a Python library that extends Jupyter and Colab notebooks with security-analyst helpers: magic commands, helper functions, and integrations with Timesketch and other DFIR tools. It exposes a shared context object so that notebook cells can pass data between analysis steps without manual serialization.

---

## Network Security

### [nogotofail](https://github.com/google/nogotofail)
**★ 2945 · `archived` · pushed 2022-10 · Apache-2.0**

nogotofail is an on-path (man-in-the-middle) network traffic security testing tool that intercepts TLS/SSL connections to detect misconfigurations: certificate validation failures, weak cipher suites, SSLv3/RC4 usage, and cleartext credential leaks. A Python daemon runs as a transparent network proxy; a Python client library reports results from the target device. The project is archived.

### [capirca](https://github.com/google/capirca)
**★ 852 · `active` · pushed 2026-05 · Apache-2.0**

Capirca is a multi-platform ACL (Access Control List) generation system. Engineers write firewall policy in a single high-level, platform-neutral language (policy files with terms and tokens); Capirca compiles them to vendor-specific ACL syntax for Cisco, Juniper, iptables, ipset, pf, Aruba, and others. This enables a single source of truth for network access policies deployed across heterogeneous infrastructure.

---

## Cryptographic Auditing

### [paranoid_crypto](https://github.com/google/paranoid_crypto)
**★ 802 · `active` · pushed 2025-06 · Apache-2.0**
Topics: `cryptography` `security`

Project Paranoid checks for well-known weaknesses in cryptographic artifacts — RSA/ECDSA public keys, digital signatures, and pseudorandom number sequences — using a library of checks drawn from published academic work (Lenstra et al. 2012, Heninger et al. 2012, Bernstein et al. 2013, Breitner & Heninger 2019, and others). It targets large-scale certificate and key corpus analysis to identify systemic generation flaws such as shared prime factors, biased ECDSA nonces, and weak entropy sources.

---

## Vulnerability & Patch Analysis

### [vanir](https://github.com/google/vanir)
**★ 359 · `active` · pushed 2026-05 · BSD-3-Clause**

Vanir is a source-code static analysis tool that identifies missing security patches in C/C++ and Java source trees. It pulls CVE signatures from the Open Source Vulnerabilities (OSV) database and matches them against the target source, reporting which Android security bulletin CVEs (2020 onwards) are absent. Designed for low false-positive rates at scale; available as `pip install vanir`.

### [mcp-security](https://github.com/google/mcp-security)
**★ 483 · `active` · pushed 2026-05 · Apache-2.0**

MCP servers that expose Google's security products — Google SecOps (Chronicle), SecOps SOAR, Google Threat Intelligence (GTI), and Security Command Center (SCC) — as Model Context Protocol tools consumable by MCP clients such as Claude Desktop. Includes both a self-hosted stdio mode and a fully managed remote MCP server. Authentication uses Google Application Default Credentials or a service account key.

### [vulncode-db](https://github.com/google/vulncode-db)
**★ 576 · `archived` · pushed 2022-01 · Apache-2.0**

Vulncode-DB was a web platform that linked CVE entries to their fixing commits in open-source repositories, providing annotated, code-level vulnerability context. The project is archived; the Open Source Vulnerabilities (OSV) database at `osv.dev` now serves as the maintained successor for structured OSS vulnerability data.

---

## Endpoint Security & Fleet Management

### [ukip](https://github.com/google/ukip)
**★ 549 · `active` · pushed 2023-07 · Apache-2.0**

ukip (USB Keystroke Injection Protection) is a Linux daemon that detects USB HID keystroke injection attacks (BadUSB/Rubber Ducky style) by monitoring inter-keystroke timing. In monitor mode it logs suspicious devices to syslog; in hardening mode it unbinds the USB driver to eject the device. The detection threshold (`KEYSTROKE_WINDOW`) and the abnormal typing speed cutoff (`ABNORMAL_TYPING`) are tunable to reduce false positives.

### [upvote_py2](https://github.com/google/upvote_py2)
**★ 449 · `archived` · pushed 2021-09 · Apache-2.0**

upvote_py2 was a multi-platform binary allowlisting (whitelisting) solution that let employees vote to approve unknown binaries for execution, backed by Santa (macOS) and Bit9 (Windows). The project is archived (Python 2); the concepts are continued in the separately maintained Santa project for macOS.

### [kernel-sanitizers](https://github.com/google/kernel-sanitizers)
**★ 470 · `active` · pushed 2025-04 · n/a**

Documentation, configuration, and scripts for Linux Kernel Sanitizers (KASan, KCSan, UBSan, and others) — fast, compiler-instrumented bug detectors for the Linux kernel. The repository collects setup guides, reproducers, and known issues to lower the barrier for kernel developers adopting sanitizer-based testing.

### [secops-wrapper](https://github.com/google/secops-wrapper)
**★ 80 · `active` · pushed 2026-04 · Apache-2.0**

A Python helper SDK that wraps the Google Security Operations (Chronicle) REST API for common use cases: ingesting logs, running UDM searches, creating detection rules, and querying assets. Available as `pip install secops` from PyPI.

---

## CTF

### [google-ctf](https://github.com/google/google-ctf)
**★ 4954 · `active` · pushed 2026-02 · Apache-2.0**
Topics: `ctf` `ctf-challenges` `google` `security`

The Google CTF repository archives most challenges from the Google Capture The Flag competition since 2017, along with the infrastructure (Docker-based challenge servers, scoring backends) needed to re-run them. Challenges span pwn, web, reversing, crypto, and misc categories at varying difficulty levels. The code in the `201x` and `202x` folders contains intentional vulnerabilities and must not be deployed to production infrastructure.

---

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [har-sanitizer](https://github.com/google/har-sanitizer) | 74 | archived | Strips credentials and sensitive headers from HTTP Archive (HAR) files |
| [python-security-manager](https://github.com/google/python-security-manager) | 36 | archived | Experimental Python security manager prototype |
| [catnip](https://github.com/google/catnip) | 26 | archived | Catnip sandbox for testing malware behavior |
| [unisim](https://github.com/google/unisim) | 147 | archived | Efficient fuzzy similarity computation and clustering |
