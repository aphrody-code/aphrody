# Google · Hardware / EDA

Open-source process design kits (PDKs) and hardware abstraction tooling developed or co-developed by Google, enabling community access to manufacturable silicon at SkyWater Technology Foundry (130nm SKY130) and GlobalFoundries (180nm GF180MCU), alongside retro hardware projects and low-level HAL utilities.

> Part of [`docs/python/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 6 repos (1 active / 5 archived).

## Process Design Kits (PDK)

### [skywater-pdk](https://github.com/google/skywater-pdk)
**★ 3535 · `archived` · pushed 2024-10 · Apache-2.0**  
Topics: `asic` `asic-library` `eda` `magic` `openram` `openroad` `pdk` `skywater`

The landmark collaboration between Google and SkyWater Technology Foundry providing the first fully open-source Process Design Kit for manufacturable silicon. Targets the SKY130 (130nm) process node. The PDK includes device models, cell libraries (standard cells, I/O pads), design rule documentation, and all files required to tape out a chip using open-source EDA tools (Magic, OpenROAD, KLayout, ngspice). Python is used throughout for scripting, cell generation, and CI/CD (including GitHub Actions via the companion `skywater-pdk-actions` repo). Documentation at `skywater-pdk.rtfd.io`. Google sponsored free multi-project wafer (MPW) shuttle runs through Efabless, enabling community chip fabrication. Archived as the PDK data itself has reached a stable state; the broader open-silicon ecosystem continues building on top of it.

### [globalfoundries-pdk-libs-gf180mcu_fd_pr](https://github.com/google/globalfoundries-pdk-libs-gf180mcu_fd_pr)
**★ 57 · `archived` · pushed 2023-08 · Apache-2.0**  
Topics: `180nm` `asic` `eda` `gf180mcu` `globalfoundries` `openroad` `pdk`

Primitive device library for the GF180MCU process node provided by GlobalFoundries. Part of Google's open-source PDK initiative extended to GlobalFoundries' 180nm node, which includes MOSFET, BJT, diode, and resistor primitives with SPICE models. Works with open-source EDA tools (OpenROAD, Magic, KLayout, ngspice). Documentation at `gf180mcu-pdk.rtfd.io`. Archived.

### [globalfoundries-pdk-libs-gf180mcu_fd_pv](https://github.com/google/globalfoundries-pdk-libs-gf180mcu_fd_pv)
**★ 14 · `archived` · pushed 2023-06 · Apache-2.0**

Physical verification rule deck library for the GF180MCU process. Contains DRC (Design Rule Check) and LVS (Layout vs. Schematic) rules for the GlobalFoundries 180nm node, enabling automated physical verification of tape-out-ready layouts. Archived.

### [skywater-pdk-libs-sky130_bag3_pr](https://github.com/google/skywater-pdk-libs-sky130_bag3_pr)
**★ 20 · `archived` · pushed 2023-05 · Apache-2.0**  
Topics: `analog` `analog-circuit` `bag` `integrated-circuits` `sky130`

BAG (BAG AMS Generator) primitives library for the SKY130 process. BAG is a Berkeley-developed framework for analog and mixed-signal (AMS) circuit generator development. This library provides SKY130-specific primitive device wrappers enabling BAG-based analog circuit generation targeting the SkyWater 130nm node. Documentation at `skywater-pdk.rtfd.io`. Archived.

---

## Hardware Abstraction / Embedded

### [copper](https://github.com/google/copper)
**★ 16 · `archived` · pushed 2019-08 · Apache-2.0**

Python module providing low-level hardware abstraction layers (HAL) as Python modules for Google hardware projects. Targets embedded and lab automation scenarios where Python scripts need to interact directly with hardware peripherals (GPIO, I2C, SPI, UART). Archived.

---

## Retro Hardware

### [myelin-acorn-electron-hardware](https://github.com/google/myelin-acorn-electron-hardware)
**★ 60 · `active` · pushed 2026-05 · Apache-2.0**

Phillip Pearson's retro hardware projects for the Acorn Electron 8-bit microcomputer, hosted under the Google org. Includes KiCad schematics, PCB layouts, and Python scripts for FPGA/CPLD programming and hardware testing. Python is used for ROM flashing and hardware verification utilities. Actively maintained as a personal project. Canonical repo at `github.com/myelin/acorn-hardware`.
