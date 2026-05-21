# Google · Data Science

Python libraries and tools for ML data loading, geospatial analysis (Earth Engine, Xarray), weather/climate data pipelines, advertising data workflows, binary protocol tooling, quantum computing clients, and general-purpose data structures.

> Part of [`docs/python/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 24 repos (9 active / 15 archived).

## ML Data Loading

### [grain](https://github.com/google/grain)
**★ 730 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `jax` `machine-learning` `python`

Python library for reading and processing data for training and evaluating JAX models. Provides a declarative pipeline API (`MapDataset`, `shuffle`, `map`, `batch`) that is flexible, fast, and deterministic. Supports multi-worker parallelism and is designed for large-scale training data pipelines. Published on PyPI as `grain`; supported on Linux and Windows (x86\_64/aarch64). Documentation at `google-grain.readthedocs.io`.

### [sedpack](https://github.com/google/sedpack)
**★ 36 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `dataset` `deep-learning`

Scalable and efficient dataset packing library for ML training. Provides a storage format and loading API optimized for large datasets accessed during deep learning training loops. Documentation at `google.github.io/sedpack/`.

---

## Geospatial / Earth Engine

### [Xee](https://github.com/google/Xee)
**★ 352 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `earthengine` `xarray`

Xarray extension (backend engine) for Google Earth Engine. Allows loading Earth Engine images and image collections directly into `xarray.Dataset` objects using the Earth Engine Python API, enabling standard scientific Python workflows (Dask, Zarr, matplotlib) on satellite imagery. Documentation at `xee.rtfd.io`.

### [xarray-beam](https://github.com/google/xarray-beam)
**★ 167 · `active` · pushed 2026-01 · Apache-2.0**  
Topics: `xarray` `beam` `dask` `zarr`

Library for distributed Xarray processing with Apache Beam. Provides `xarray_beam.DatasetGraph` and related primitives for expressing large-scale Xarray transformations as Beam pipelines, enabling cloud-scale geospatial and climate data processing. Documentation at `xarray-beam.readthedocs.io`.

### [xarray-tensorstore](https://github.com/google/xarray-tensorstore)
**★ 66 · `active` · pushed 2026-05 · Apache-2.0**

Xarray backend for TensorStore, Google's library for reading and writing large multi-dimensional arrays. Enables zero-copy access to TensorStore arrays as Xarray DataArrays, useful for large ML checkpoints and satellite data stored in Zarr/N5 formats.

---

## Weather / Climate

### [weather-tools](https://github.com/google/weather-tools)
**★ 249 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `apache-beam` `python` `weather`

Tools for making weather and climate data accessible and useful. Provides Apache Beam-based pipelines for downloading, subsetting, and converting numerical weather prediction (NWP) data (GRIB2, NetCDF) into cloud-friendly formats (BigQuery, Zarr). Documentation at `weather-tools.readthedocs.io/`.

---

## Data Structures

### [pygtrie](https://github.com/google/pygtrie)
**★ 820 · `archived` · pushed 2021-04 · Apache-2.0**

Pure-Python trie (prefix tree) data structure library. Supports character-level and path-level tries, with subtrie views and longest-prefix-match operations. Widely used in network prefix matching and autocomplete applications. Archived; library reached feature-complete status.

---

## Binary Protocol / Embedded

### [emboss](https://github.com/google/emboss)
**★ 86 · `active` · pushed 2026-05 · Apache-2.0**

Domain-specific language and code generator for reading and writing binary data structures. Emboss `.emb` schema files describe binary layouts; the compiler generates Python (and C++) accessor code with bounds checking. Particularly suited for firmware communication protocols, hardware registers, and network packet formats. Actively developed.

---

## Advertising Data / Workflows

### [starthinker](https://github.com/google/starthinker)
**★ 174 · `archived` · pushed 2024-04 · Apache-2.0**  
Topics: `bigquery` `airflow` `google-ads` `dv360` `cm360` `google-analytics` `data-science` `python`

Reference framework for building advertising data workflows on GCP. Accelerates authentication, logging, scheduling, and deployment for solutions using BigQuery, DV360, CM360, Google Ads, and Analytics. Includes 50+ pre-built tasks deployable via Airflow, Cloud Functions, or Colab. Archived.

### [megalista](https://github.com/google/megalista)
**★ 143 · `archived` · pushed 2025-01 · Apache-2.0**

First-party data integration solution for marketing teams. Ingests audience and conversion data from BigQuery and uploads it to Google Ads, Campaign Manager, and Google Analytics via Apache Beam pipelines on Cloud Dataflow. Archived.

### [orchestra](https://github.com/google/orchestra)
**★ 51 · `archived` · pushed 2020-09 · Apache-2.0**

Advertising data lakes and workflow automation using Apache Airflow. Provides Airflow operators and hooks for orchestrating data pipelines across Google Marketing Platform APIs (DV360, Campaign Manager). Archived.

---

## LLM / AI Research

### [sycophancy-intervention](https://github.com/google/sycophancy-intervention)
**★ 121 · `archived` · pushed 2023-08 · Apache-2.0**

Scripts for generating synthetic fine-tuning data to reduce sycophancy in large language models. Companion to the paper "Towards Understanding Sycophancy in Language Models" (arxiv:2308.03958). Generates comparison pairs where a sycophantic response is contrasted with a factually accurate one. Archived.

---

## Transit Data

### [transitfeed](https://github.com/google/transitfeed)
**★ 690 · `archived` · pushed 2022-09 · Apache-2.0**

Python library for reading, validating, and writing transit schedule data in the GTFS (General Transit Feed Specification) format. Used by transit agencies and researchers to work with public transit timetables, stop locations, and route information. The canonical Python GTFS implementation; archived as the GTFS ecosystem has migrated to other tooling.

---

## Quantum Computing

### [floq-client](https://github.com/google/floq-client)
**★ 19 · `archived` · pushed 2021-09 · Apache-2.0**

Client library for Google's Floq quantum simulation service (high-performance quantum circuit simulation on TPUs). Provides a Cirq-compatible interface for submitting quantum circuits to the Floq backend. Archived as Floq was discontinued.

---

## PCB / Hardware Design

### [pcbdl](https://github.com/google/pcbdl)
**★ 190 · `archived` · pushed 2021-04 · Other**  
Topics: `eda` `electronics` `hardware` `hdl` `netlist` `python` `schematics`

PCB Design Language: a Python-based programming approach to schematic design. Schematics are expressed as Python code, enabling version control, parameterization, and automated design rule checking. Generates netlists and schematic visualizations. Archived.

---

## Molecular Simulation

### [differentiable-atomistic-potentials](https://github.com/google/differentiable-atomistic-potentials)
**★ 56 · `archived` · pushed 2018-07 · Apache-2.0**

Automatically differentiable atomistic potentials implemented in TensorFlow for molecular dynamics simulations. Enables gradient-based optimization of empirical interatomic potentials (Lennard-Jones, EMT). Archived; predates JAX-based successors in the atomistic ML space.

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [google-visualization-python](https://github.com/google/google-visualization-python) | 134 | archived | Python bindings for Google Charts / Visualization API |
| [memcache-collections](https://github.com/google/memcache-collections) | 67 | archived | Concurrent distributed data structures on top of memcache |
| [shopping-markup](https://github.com/google/shopping-markup) | 48 | archived | Data-driven insights for retail / Shopping Ads |
| [dqm](https://github.com/google/dqm) | 26 | archived | Data quality monitoring platform for online advertising |
| [waze-ccp-gcp](https://github.com/google/waze-ccp-gcp) | 11 | archived | Waze CCP JSON feed -> BigQuery GIS tables + GeoJSON |
| [genomics-protos](https://github.com/google/genomics-protos) | 12 | archived | Protobuf schemas for Google Genomics API |
| [ai-weather-climate](https://github.com/google/ai-weather-climate) | 17 | archived | AI/ML for weather and climate (no description) |
| [sprockets](https://github.com/google/sprockets) | 11 | archived | State-transition conformance testing framework |
