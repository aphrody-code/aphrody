# Google · Mobile / Android

Python tooling for the Android ecosystem: ADB and Fastboot protocol implementations, Android emulator container orchestration, file synchronization over ADB, automated device testing utilities, and AIY (AI-on-Raspberry-Pi) maker kits.

> Part of [`docs/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 12 repos (4 active / 8 archived).

## Android Emulator

### [android-emulator-container-scripts](https://github.com/google/android-emulator-container-scripts)
**★ 2037 · `active` · pushed 2026-05 · Apache-2.0**

Minimal scripts for running the Android emulator in Docker containers on Linux systems. Requires Python 3.10+, ADB, Docker (compose v2 or legacy v1), and KVM (bare-metal or nested virtualization). Pre-built images are hosted in a public registry; containers can be pulled and run without a local build step. Supports WebRTC-based remote display streaming for headless CI environments. Requires KVM, so macOS and Windows Docker are unsupported.

---

## ADB / Fastboot

### [python-adb](https://github.com/google/python-adb)
**★ 1859 · `archived` · pushed 2024-04 · Apache-2.0**

Pure-Python implementation of the ADB and Fastboot protocols using `libusb1` for USB communication. Provides `pyadb` and `pyfastboot` CLI tools and a library API. Operates without the ADB daemon (`adbd`), making it suitable for automated testing scenarios requiring per-device isolation. Does not support concurrent commands to the same device. Maintained by ex-Google engineers; archived status but still receiving minor updates. Note: `adb_shell` is recommended for new projects.

### [adb-sync](https://github.com/google/adb-sync)
**★ 1094 · `archived` · pushed 2024-03 · Apache-2.0**

File synchronization tool between a host machine and Android devices over ADB. Mirrors `rsync`-style semantics (sync directories bidirectionally) using ADB file transfer. Archived; `adb sync` built into the Android SDK covers the core use case, but this tool offered finer-grained control.

---

## AIY Projects (Raspberry Pi)

### [aiyprojects-raspbian](https://github.com/google/aiyprojects-raspbian)
**★ 1662 · `archived` · pushed 2021-12 · Apache-2.0**  
Topics: `ai` `maker` `python` `raspberry-pi`

API libraries, code samples, and system images for Google's AIY Projects: the Voice Kit (speech recognition and synthesis on Raspberry Pi using Google Assistant SDK) and the Vision Kit (image classification using the Intel Movidius/Coral Edge TPU). The canonical code repository for the AIY hardware kits sold by Google. Archived as production support ended; community forks are active.

### [aiyprojects-raspbian-tools](https://github.com/google/aiyprojects-raspbian-tools)
**★ 12 · `archived` · pushed 2021-08 · Apache-2.0**

Companion tooling repository for `aiyprojects-raspbian`. Contains build scripts, image configuration, and deployment utilities for creating AIY Raspbian OS images. Archived alongside the main AIY projects repository.

---

## Android Testing / Utilities

### [fplutil](https://github.com/google/fplutil)
**★ 333 · `archived` · pushed 2018-07 · Apache-2.0**

Small libraries and tools for Android application development, primarily targeting Google's FPL (Fun Propulsion Labs) game development toolchain. Includes Python build utilities, Android test runner helpers, and APK deployment scripts. Archived.

### [lab_device_proxy](https://github.com/google/lab_device_proxy)
**★ 39 · `archived` · pushed 2014-04 · BSD-3-Clause**

Python daemon that exposes ADB and idevice (iOS) commands on a remote host over a network socket. Enables lab automation setups where devices are physically attached to one machine but test scripts run on another. Archived.

### [mobly-android-screen-recorder](https://github.com/google/mobly-android-screen-recorder)
**★ 6 · `active` · pushed 2026-01 · Apache-2.0**

Mobly controller plugin for recording Android device screens during automated tests. Integrates with the Mobly test framework (Google's Python-based Android/IoT test framework) to capture screen recordings as test artifacts. Actively maintained.

### [android-beat](https://github.com/google/android-beat)
**★ 6 · `active` · pushed 2026-03 · Apache-2.0**

Recently created (2025-11) active repository in the Android tooling space. No public description. Actively receiving commits as of 2026-03.

---

## Wear OS / Android Accessories

### [android-wear-stitch-script](https://github.com/google/android-wear-stitch-script)
**★ 39 · `archived` · pushed 2019-05 · Apache-2.0**

Scripts for stitching together Wear OS (Android Wear) screenshots and UI elements for documentation or design verification purposes. Archived.

---

## Android / Education

### [coursebuilder-android-container-module](https://github.com/google/coursebuilder-android-container-module)
**★ 22 · `archived` · pushed 2017-10 · Apache-2.0**

Android container module for Google Course Builder (an open-source online education platform). Provided Android-specific integration for delivering course content. Archived.

### [android-cuttlefish-authentication](https://github.com/google/android-cuttlefish-authentication)
**★ 4 · `archived` · pushed 2021-09 · Apache-2.0**

Authentication module for Android Cuttlefish (Google's reference virtual Android device). Archived; authentication was integrated directly into the main Cuttlefish project.
