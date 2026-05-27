# Google · Web / Systems / Networking

Python tools and libraries for network protocol testing, SSL/TLS inspection, transport benchmarking, protobuf tooling, USB/Bluetooth device control, transit feed processing, and web infrastructure utilities.

> Part of [`docs/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 22 repos (7 active / 15 archived).

## Networking — Protocol Testing / Management

### [gnxi](https://github.com/google/gnxi)
**★ 286 · `active` · pushed 2026-03 · Apache-2.0**  
Topics: `gnmi` `gnmi-client` `gnmi-protocol` `gnmi-target` `gnoi` `gnoi-client` `gnoi-protocols` `gnoi-target`

gNXI (gRPC Network Management/Operations Interface) Tools: a collection of Python and Go utilities for interacting with network devices via gNMI (gRPC Network Management Interface) and gNOI (gRPC Network Operations Interface). Provides reference implementations of gNMI client/target and gNOI client/target for testing and validating network device compliance. Actively maintained.

### [testrun](https://github.com/google/testrun)
**★ 52 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `automation` `iot` `network` `security`

Automated verification framework for network-based device behavior, targeting IoT device compliance testing. Orchestrates network capture, protocol analysis, and behavioral assertions against devices under test. Designed for security and certification workflows where device network behavior must be validated against a specification.

### [transperf](https://github.com/google/transperf)
**★ 213 · `archived` · pushed 2021-05 · Apache-2.0**  
Topics: `bbr` `bbr2` `networking` `tcp` `testing`

Tool for testing transport protocol performance (especially BBR and BBR2 congestion control algorithms) over emulated network scenarios using Linux traffic control (`tc netem`). Used by Google's networking team to validate TCP stack behavior under controlled loss and delay conditions. Archived.

---

## Security / TLS Inspection

### [ssl_logger](https://github.com/google/ssl_logger)
**★ 1116 · `archived` · pushed 2020-10 · Apache-2.0**

Frida-based tool that decrypts and logs a process's SSL/TLS traffic by hooking OpenSSL/BoringSSL functions at runtime. Operates without a MITM proxy — hooks directly into the process's memory to capture plaintext before encryption and after decryption. Widely referenced in mobile and desktop application security research. Archived.

---

## Data Serialization / Protobuf

### [protobuf-extensibility-for-burp](https://github.com/google/protobuf-extensibility-for-burp)
**★ 93 · `active` · pushed 2024-06 · Apache-2.0**

Burp Suite extension for decoding and manipulating protobuf-encoded HTTP payloads during web application security testing. Provides automatic protobuf detection and a UI for editing decoded message fields within Burp's interceptor. Useful for auditing gRPC-over-HTTP/1.1 and binary protobuf REST APIs.

---

## USB / U2F / Bluetooth

### [pyu2f](https://github.com/google/pyu2f)
**★ 84 · `archived` · pushed 2025-02 · Apache-2.0**

Pure-Python U2F (Universal 2nd Factor) host library supporting Linux, Windows, and macOS. Communicates with U2F hardware tokens over USB HID, implementing the FIDO U2F host-side protocol for registration and authentication. Used as a dependency in Google Cloud SDK's `gcloud auth login` with security key support. Archived.

### [python-laurel](https://github.com/google/python-laurel)
**★ 60 · `archived` · pushed 2020-11 · Apache-2.0**

Python library for controlling C by GE Bluetooth smart bulbs. Implements the proprietary BLE protocol for the C by GE light bulb line, enabling programmatic control of brightness, color temperature, and scheduling. Archived.

### [python-dimond](https://github.com/google/python-dimond)
**★ 33 · `archived` · pushed 2018-12 · Apache-2.0**

Python implementation of the Telink Bluetooth mesh protocol. Allows controlling Telink-based Bluetooth mesh devices (lights, switches) from a Python host. Low-level BLE mesh stack implementation. Archived.

---

## Data Structures / Libraries

### [TensorNetwork](https://github.com/google/TensorNetwork)
**★ 1866 · `archived` · pushed 2023-09 · Apache-2.0**  
Topics: `tensor-networks` `matrix-product-states`

Python library for easy and efficient manipulation of tensor networks — mathematical structures used in quantum physics simulations and certain ML architectures. Supports multiple backends (NumPy, TensorFlow, JAX, PyTorch) for contraction operations. Widely cited in quantum computing and condensed matter physics research. Archived; active development moved to specialized downstream projects.

### [python_portpicker](https://github.com/google/python_portpicker)
**★ 153 · `archived` · pushed 2023-08 · Apache-2.0**

Python module for finding available network ports for testing. Provides a reliable way to allocate free TCP ports across processes without race conditions, using file locking. Published on PyPI as `portpicker`; used extensively in Google's Python test infrastructure. Archived.

---

## ML Distributed Checkpointing

### [ml-flashpoint](https://github.com/google/ml-flashpoint)
**★ 15 · `active` · pushed 2026-05 · Apache-2.0**

Memory-first, high-speed distributed checkpointing library for ML training workloads. Designed to minimize checkpoint latency by staging data in host memory before flushing to persistent storage. Documentation at `google.github.io/ml-flashpoint`. Created 2026-01; actively developed.

---

## Web / HTML Utilities

### [pre-commit-tool-hooks](https://github.com/google/pre-commit-tool-hooks)
**★ 37 · `active` · pushed 2024-05 · Apache-2.0**

Collection of pre-commit hooks for enforcing code quality and consistency standards. Includes hooks for checking Python docstrings, YAML formatting, and other style conventions. Designed to integrate with the `pre-commit` framework.

### [html-to-jsonld-converter](https://github.com/google/html-to-jsonld-converter)
**★ 5 · `active` · pushed 2025-03 · Apache-2.0**

Tool for extracting structured data from HTML pages and converting it to JSON-LD format. Targets schema.org markup extraction for SEO and knowledge graph use cases.

---

## Proxies / API Gateways

### [magic-github-proxy](https://github.com/google/magic-github-proxy)
**★ 150 · `archived` · pushed 2022-09 · Apache-2.0**

Stateless, access-limiting proxy for the GitHub API. Issues scoped GitHub tokens to downstream services without exposing the root credentials, enforcing per-service permission restrictions. Useful in CI/CD systems where multiple pipelines need GitHub API access with least-privilege tokens. Archived.

---

## Cryptography

### [jws](https://github.com/google/jws)
**★ 62 · `archived` · pushed 2020-09 · Apache-2.0**

Python implementation of JSON Web Signature (JWS / RFC 7515). Provides signing and verification of JSON payloads using RSA and ECDSA algorithms. Archived.

### [bi-tempered-loss](https://github.com/google/bi-tempered-loss)
**★ 147 · `archived` · pushed 2021-12 · Apache-2.0**

Implementation of the Robust Bi-Tempered Logistic Loss (arxiv:1906.03361), a loss function based on Bregman divergences that is more resistant to noisy labels than standard cross-entropy. Provides TensorFlow and JAX implementations. Archived.

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [ffn](https://github.com/google/ffn) | 348 | active | Flood-Filling Networks for 3D instance segmentation in connectomics |
| [google-apputils](https://github.com/google/google-apputils) | 38 | archived | Legacy app utilities; superseded by abseil-py |
| [ga-serverless-streaming](https://github.com/google/ga-serverless-streaming) | 14 | archived | Google Analytics serverless streaming pipeline |
| [campaign-manager-bulk-uploader](https://github.com/google/campaign-manager-bulk-uploader) | 10 | archived | Campaign Manager 360 bulk trafficking |
| [skywater-pdk-actions](https://github.com/google/skywater-pdk-actions) | 16 | archived | GitHub Actions for SkyWater PDK CI |
| [py-html-contextual-escaping](https://github.com/google/py-html-contextual-escaping) | 4 | archived | Contextual HTML escaping library |
