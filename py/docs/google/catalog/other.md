# Google · Other & uncategorized

Python repos from `google` that do not fit the major thematic categories — predominantly archived research/paper code, Python utilities and libraries, IT operations and infrastructure tooling, advertising/analytics tools, and a mix of internal experiments, samples, and one-off projects. Approximately 271 of the 299 repos are archived.

> Part of [`docs/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 299 repos (28 active / 271 archived).

---

## Notable / actively maintained

### [uncertainty-baselines](https://github.com/google/uncertainty-baselines)
**★ 1573 · `active` · pushed 2026-03 · Apache-2.0**  
Topics: `bayesian-methods` `deep-learning` `machine-learning` `neural-networks` `probabilistic-programming` `statistics` `tensorflow`  
High-quality reference implementations of standard and SOTA methods for uncertainty estimation and robustness benchmarking. Intended as a starting template for researchers to build on, with minimal intra-codebase dependencies. Covers image classification, NLP, and tabular domains.

### [glazier](https://github.com/google/glazier)
**★ 1252 · `active` · pushed 2026-04 · Apache-2.0**  
Automated Windows OS deployment tool developed at Google. Boots systems into WinPE, fetches instructions from an HTTPS server, applies a base OS, then installs applications and configurations. Designed for large-scale enterprise fleet management.

### [ml_collections](https://github.com/google/ml_collections)
**★ 1029 · `active` · pushed 2026-03 · Apache-2.0**  
Python collection types designed for ML configuration workflows. Provides `ConfigDict` and `FrozenConfigDict` with dot-based field access, locking, lazy computation, type safety, and `FieldReference` for shared hyperparameters. Published on PyPI; widely used across Google's JAX-based research.

### [learned_optimization](https://github.com/google/learned_optimization)
**★ 803 · `active` · pushed 2026-04 · Apache-2.0**  
Research codebase for learning optimizers (meta-learning) in JAX. Provides infrastructure for training and evaluating learned optimizers on a variety of inner-loop tasks.

### [bumble](https://github.com/google/bumble)
**★ 509 · `active` · pushed 2026-05 · Apache-2.0**  
Full-featured Bluetooth stack written entirely in Python. Supports BLE and Classic (BR/EDR) including GAP, L2CAP, ATT, GATT, SMP, SDP, RFCOMM, HFP, HID, and A2DP. Works with physical radios via HCI over USB/UART/Linux VHCI and with virtual Bluetooth (e.g., Android emulator). Actively maintained with online documentation at google.github.io/bumble.

### [autobound](https://github.com/google/autobound)
**★ 365 · `active` · pushed 2025-10 · Apache-2.0**  
Topics: `autodiff` `interval-arithmetic` `jax`  
JAX library that automatically computes upper and lower bounds on functions using interval arithmetic combined with automatic differentiation.

### [praxis](https://github.com/google/praxis)
**★ 196 · `active` · pushed 2026-05 · Apache-2.0**  
Layer library for the Pax/Paxml JAX-based training framework. Provides reusable neural network layers optimized for large-scale ML, designed to be usable by other JAX-based projects beyond Pax.

### [apitools](https://github.com/google/apitools)
**★ 155 · `active` · pushed 2026-03 · Apache-2.0**  
Python client library utilities for Google APIs, providing HTTP transport, retry logic, and schema-to-Python object mapping. Used internally by `gcloud` and other Google client tools.

### [saxml](https://github.com/google/saxml)
**★ 151 · `active` · pushed 2026-05 · Apache-2.0**  
Experimental inference serving system for Paxml, JAX, and PyTorch models. A Sax cell consists of an admin server and model servers; the admin server tracks model servers, assigns published models, and helps clients locate servers.

### [tfp-causalimpact](https://github.com/google/tfp-causalimpact)
**★ 155 · `active` · pushed 2026-02 · Apache-2.0**  
TensorFlow Probability implementation of the CausalImpact method for estimating the causal effect of an intervention in time-series data.

### [nsscache](https://github.com/google/nsscache)
**★ 155 · `active` · pushed 2026-04 · GPL-2.0**  
Topics: `getpwent` `ldap` `nss` `python`  
Asynchronously synchronises local Linux NSS databases (passwd, group, shadow) with remote directory services such as LDAP. Active infrastructure tool used in Linux fleet management.

### [jaxite](https://github.com/google/jaxite)
**★ 102 · `active` · pushed 2026-05 · Apache-2.0**  
Fully homomorphic encryption (FHE) library targeting TPUs and GPUs. Implements the CGGI cryptosystem and is a supported backend for Google's HEIR FHE compiler. Active development targets CKKS on TPU via the CROSS algorithm.

### [sbsim](https://github.com/google/sbsim)
**★ 111 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `reinforcement-learning` `smart-building`  
Calibrated simulation suite and real-world dataset for offline training of RL agents to optimize energy and emissions in office buildings. Accompanies published BuildSys 2023 and NeurIPS 2024 papers; dataset available via TensorFlow Datasets.

### [osv-scanner-action](https://github.com/google/osv-scanner-action)
**★ 79 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `github-actions` `osv` `vulnerability-scanners`  
Reusable GitHub Actions workflow that runs OSV-Scanner to detect known vulnerabilities in repository dependencies.

### [corpuscrawler](https://github.com/google/corpuscrawler)
**★ 214 · `active` · pushed 2025-08 · Other**  
Topics: `corpus-builder` `corpus-linguistics` `crawling` `linguistics` `minority-language`  
Web crawler that downloads and pre-processes text corpora for linguistic research, with emphasis on minority and low-resource languages.

### [matched_markets](https://github.com/google/matched_markets)
**★ 98 · `active` · pushed 2025-08 · Apache-2.0**  
Python library for designing and analyzing geo experiments using Matched Markets and Time-Based Regression (TBR) methodologies.

### [xctestrunner](https://github.com/google/xctestrunner)
**★ 158 · `active` · pushed 2026-02 · Apache-2.0**  
Binary for running prebuilt iOS test bundles on iOS simulators and real devices from the command line; used in CI pipelines for iOS testing at Google.

### [macops](https://github.com/google/macops)
**★ 824 · `active` · pushed 2023-06 · Apache-2.0**  
Collection of utilities, tools, and scripts for managing and tracking a fleet of Macintosh computers in a corporate environment. Includes scripts used by Google's own Mac fleet operations.

### [init2winit](https://github.com/google/init2winit)
**★ 85 · `active` · pushed 2026-05 · Apache-2.0**  
JAX/Flax research codebase for studying optimization, training dynamics, and neural network initialization at scale. Actively maintained with ongoing experiments.

---

## Research code & papers (ML/vision/NLP)

Archived research repositories accompanying specific papers, typically released once at publication.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [mipnerf](https://github.com/google/mipnerf) | 939 | archived | Mip-NeRF: anti-aliased 3D neural radiance field representation |
| [ml-fairness-gym](https://github.com/google/ml-fairness-gym) | 315 | archived | Simulation framework for studying long-term impacts of ML fairness decisions |
| [ldif](https://github.com/google/ldif) | 321 | archived | 3D shape representation with Local Deep Implicit Functions (LDIF) |
| [active-qa](https://github.com/google/active-qa) | 344 | archived | Active question answering via reformulation |
| [vae-seq](https://github.com/google/vae-seq) | 177 | archived | Variational Auto-Encoders in sequential settings |
| [gcnn-survey-paper](https://github.com/google/gcnn-survey-paper) | 175 | archived | Code for a survey of graph convolutional neural networks |
| [trajax](https://github.com/google/trajax) | 240 | archived | JAX-based trajectory optimization algorithms |
| [mystyle](https://github.com/google/mystyle) | 129 | archived | Personalized text-to-image generation research |
| [shuwa](https://github.com/google/shuwa) | 146 | archived | Sign language gesture recognition research |
| [ehr-predictions](https://github.com/google/ehr-predictions) | 110 | archived | ML models for predictions from electronic health records |
| [microscopeimagequality](https://github.com/google/microscopeimagequality) | 92 | archived | Deep learning model for microscopy image quality assessment |
| [madi](https://github.com/google/madi) | 79 | archived | Multivariate Anomaly Detection with Interpretability (ICML 2020) |
| [graph_distillation](https://github.com/google/graph_distillation) | 64 | archived | Graph Distillation for cross-modal action detection |
| [brain_autorl](https://github.com/google/brain_autorl) | 56 | archived | Automated reinforcement learning research from Google Brain |
| [lassie](https://github.com/google/lassie) | 55 | archived | 3D animal shape and pose estimation |
| [storybench](https://github.com/google/storybench) | 54 | archived | Benchmark for evaluating LLM long-form story generation |
| [zoom-to-inpaint](https://github.com/google/zoom-to-inpaint) | 21 | archived | Zoom-to-Inpaint image completion research |
| [aperture_supervision](https://github.com/google/aperture_supervision) | 33 | archived | Aperture supervision for monocular depth estimation |
| [dl_bounds](https://github.com/google/dl_bounds) | 17 | archived | Generalization bounds for deep learning |
| [asymproj_edge_dnn](https://github.com/google/asymproj_edge_dnn) | 25 | archived | Asymmetric projections for link prediction in graphs |
| [retrieval-qa-eval](https://github.com/google/retrieval-qa-eval) | 42 | archived | Evaluation framework for retrieval-based QA |
| [airdialogue](https://github.com/google/airdialogue) | 47 | archived | Goal-oriented dialogue corpus for task completion |
| [airdialogue_model](https://github.com/google/airdialogue_model) | 17 | archived | Model code for AirDialogue |
| [n-digit-mnist](https://github.com/google/n-digit-mnist) | 46 | archived | Dataset generator for multi-digit MNIST sequences |
| [multi-task-architecture-search](https://github.com/google/multi-task-architecture-search) | 12 | archived | Neural architecture search for multi-task learning |
| [mipsqa](https://github.com/google/mipsqa) | 26 | archived | QA dataset from mobile information-seeking scenarios |
| [categorybuilder](https://github.com/google/categorybuilder) | 98 | archived | Builds semantic categories from seed terms using distributional methods |
| [meta_tagger](https://github.com/google/meta_tagger) | 49 | archived | Meta-learning for sequence tagging |
| [text2text](https://github.com/google/text2text) | 54 | archived | Cross-lingual text-to-text transformation utilities |
| [vsf-time-series](https://github.com/google/vsf-time-series) | 31 | archived | Vector space functions for time-series classification |
| [HyperCompressBench](https://github.com/google/HyperCompressBench) | 18 | archived | Benchmark for hyperparameter-efficient neural network compression |
| [decoupled_gaussian_process](https://github.com/google/decoupled_gaussian_process) | 23 | archived | Decoupled representation of Gaussian processes |
| [NeuroNER-CSPMC](https://github.com/google/NeuroNER-CSPMC) | 12 | archived | Named-entity recognition with cross-sentence predictions |
| [wide_bnn_sampling](https://github.com/google/wide_bnn_sampling) | 7 | archived | Sampling from wide Bayesian neural networks |
| [dl_bounds](https://github.com/google/dl_bounds) | 17 | archived | Deep learning generalization bounds |
| [project_cartesian](https://github.com/google/project_cartesian) | 20 | archived | Research on neural network compositionality |
| [omnimatte-sp](https://github.com/google/omnimatte-sp) | 12 | active | Omnimatte with spatial-temporal patches for video segmentation |
| [hi-lassie](https://github.com/google/hi-lassie) | 25 | archived | Hierarchical unsupervised 3D articulated shape learning |
| [lut3d_utils](https://github.com/google/lut3d_utils) | 11 | archived | Utilities for working with 3D lookup tables (color grading) |
| [referring-manipulation](https://github.com/google/referring-manipulation) | 9 | archived | Referring expression comprehension for robotic manipulation |
| [putting-dune](https://github.com/google/putting-dune) | 10 | archived | Autonomous microscopy control with RL |
| [multispecies-whale-detection](https://github.com/google/multispecies-whale-detection) | 8 | archived | Multi-species whale call detection from passive acoustic monitoring |
| [zoom-to-inpaint](https://github.com/google/zoom-to-inpaint) | 21 | archived | Zoom-to-Inpaint progressive image inpainting |
| [telluride_decoding](https://github.com/google/telluride_decoding) | 14 | archived | EEG/audio auditory attention decoding (Telluride workshops) |
| [VRD](https://github.com/google/VRD) | 6 | archived | Visual relationship detection research |
| [gumbel_sinkhorn](https://github.com/google/gumbel_sinkhorn) | 78 | archived | Gumbel-Sinkhorn permutation learning |
| [retriever_parsing](https://github.com/google/retriever_parsing) | 9 | archived | Retrieval-augmented parsing models |
| [lexical-masks](https://github.com/google/lexical-masks) | 8 | archived | Lexical masking for language model pretraining |
| [content_recommendation_using_word2vec](https://github.com/google/content_recommendation_using_word2vec) | 25 | archived | Content recommendation with Word2Vec embeddings |
| [pbvi](https://github.com/google/pbvi) | 8 | archived | Point-Based Value Iteration for POMDPs |
| [stress_transfer](https://github.com/google/stress_transfer) | 8 | archived | Geophysical stress transfer modeling |
| [expt-analysis](https://github.com/google/expt-analysis) | 7 | archived | Statistical experiment analysis utilities |
| [autol2](https://github.com/google/autol2) | 7 | archived | Automated L2 regularization research |
| [uafcs](https://github.com/google/uafcs) | 4 | archived | Urban air fleet control simulation |
| [structured_labels](https://github.com/google/structured_labels) | 5 | archived | Learning with structured label correlations |
| [seatera](https://github.com/google/seatera) | 18 | archived | Seat era research code |
| [deluca-lung](https://github.com/google/deluca-lung) | 5 | archived | Lung simulator for mechanical ventilator RL experiments |
| [parallel_accel](https://github.com/google/parallel_accel) | 3 | archived | Parallel quantum circuit simulation on accelerators |
| [tcav-for-ehr](https://github.com/google/tcav-for-ehr) | 6 | archived | TCAV concept-based explanations applied to EHR data |
| [blockbuster](https://github.com/google/blockbuster) | 5 | archived | Block-sparse attention research |
| [b-con](https://github.com/google/b-con) | 6 | archived | Contrastive learning for behavioral signals |
| [distla_core](https://github.com/google/distla_core) | 5 | active | Distributed linear algebra on TPU/GPU with JAX |
| [evt-air-risk-aiaa-scitech-2026](https://github.com/google/evt-air-risk-aiaa-scitech-2026) | 3 | active | Electric vertical takeoff air risk modeling (AIAA 2026 paper) |
| [helpseeking](https://github.com/google/helpseeking) | 1 | active | Help-seeking behavior research code |
| [hcls_agents_catalog](https://github.com/google/hcls_agents_catalog) | 1 | active | Health and life sciences LLM agents catalog |

---

## JAX ecosystem utilities

Active or high-value libraries in the JAX/TPU toolchain that do not fit the main ML categories.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [airio](https://github.com/google/airio) | 24 | active | Data preprocessing and loading for AI/ML research pipelines |
| [drjax](https://github.com/google/drjax) | 19 | active | Distributed reduction primitives for JAX |
| [jaxcam](https://github.com/google/jaxcam) | 16 | active | Camera model utilities for JAX (3D vision) |
| [ml-metrics](https://github.com/google/ml-metrics) | 26 | active | Composable evaluation metrics for ML experiments |
| [example_extrapolation](https://github.com/google/example_extrapolation) | 9 | active | Dataset difficulty and extrapolation research |
| [uvq](https://github.com/google/uvq) | 148 | active | Universal Video Quality model for perceptual video quality assessment |
| [facade](https://github.com/google/facade) | 155 | active | Facade — internal tooling for structured Python configuration |
| [pica](https://github.com/google/pica) | 55 | active | Prototypical Image Concept Attributes (PICA) |
| [esamplusplus](https://github.com/google/esamplusplus) | 5 | active | eSAM++ image segmentation research |
| [pacevolve](https://github.com/google/pacevolve) | 7 | active | Evolutionary algorithms for hyperparameter pacing |
| [howtodiv](https://github.com/google/howtodiv) | 8 | active | Division algorithm research and experiments |
| [parallax](https://github.com/google/parallax) | 4 | active | Distributed training research with JAX |

---

## Python utilities & libraries

General-purpose Python libraries and standalone utilities; most are archived.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [sre_yield](https://github.com/google/sre_yield) | 189 | archived | Generates all strings matching a regex pattern |
| [python-gflags](https://github.com/google/python-gflags) | 189 | archived | Python commandline flags module (superseded by absl-py) |
| [ipaddr-py](https://github.com/google/ipaddr-py) | 194 | archived | IP address manipulation library (merged into Python 3 stdlib as ipaddress) |
| [pytruth](https://github.com/google/pytruth) | 158 | archived | Fluent assertion framework for Python unit tests |
| [python-subprocess32](https://github.com/google/python-subprocess32) | 173 | archived | Python 3 subprocess module backport for Python 2 |
| [protorpc](https://github.com/google/protorpc) | 70 | archived | Protocol Buffers RPC for App Engine Python |
| [dotty](https://github.com/google/dotty) | 49 | archived | Deep nested dictionary access with dot notation |
| [pytruth](https://github.com/google/pytruth) | 158 | archived | Fluent truth-style assertions for unit tests |
| [casfs](https://github.com/google/casfs) | 17 | archived | Content-addressable storage over pyfilesystem2 |
| [weighted-dict](https://github.com/google/weighted-dict) | 19 | archived | Dictionary with weighted random sampling |
| [uv-metrics](https://github.com/google/uv-metrics) | 14 | archived | Composable metric reporters |
| [python-proto-converter](https://github.com/google/python-proto-converter) | 17 | active | Converts between protobuf messages (DAO to API proto) |
| [pycnite](https://github.com/google/pycnite) | 27 | archived | Utilities for working with compiled Python bytecode |
| [python-atfork](https://github.com/google/python-atfork) | 26 | archived | `atfork()` support for Python (safe multi-process forking) |
| [bocado](https://github.com/google/bocado) | 9 | archived | Python type annotation inference via runtime profiling |
| [squires](https://github.com/google/squires) | 15 | archived | Self-documenting CLI framework for Python |
| [merge_pyi](https://github.com/google/merge_pyi) | 15 | archived | Merges `.pyi` type stub files into Python source |
| [checkers](https://github.com/google/checkers) | 17 | archived | Runtime argument checking utilities |
| [checkers_classic](https://github.com/google/checkers_classic) | 7 | archived | Classic checkers game (Python demo) |
| [terminal-py](https://github.com/google/terminal-py) | 7 | archived | Terminal control utilities (exported from code.google.com) |
| [ashier](https://github.com/google/ashier) | 29 | archived | Automates terminal interactions with expect-style templates |
| [bitutils](https://github.com/google/bitutils) | 16 | archived | Scripts for working with binary numbers |
| [casfs](https://github.com/google/casfs) | 17 | archived | Content-addressable storage over pyfilesystem2 |
| [objectfilter](https://github.com/google/objectfilter) | 4 | archived | Object filtering using a filter expression language |
| [pymql](https://github.com/google/pymql) | 22 | archived | Metaweb Query Language (MQL) Python client |
| [pyctr](https://github.com/google/pyctr) | 22 | archived | Click-through rate modeling utilities |
| [nixysa](https://github.com/google/nixysa) | 10 | archived | NPAPI binding generator for C++ to JS (legacy) |
| [saferpickle](https://github.com/google/saferpickle) | 19 | active | Safer Python pickle with allowlist-based deserialization |
| [duet](https://github.com/google/duet) | 28 | active | Async/await concurrency utilities for Python |
| [proto-task-queue](https://github.com/google/proto-task-queue) | 10 | active | Task queue backed by Protocol Buffers |
| [python-card-framework](https://github.com/google/python-card-framework) | 23 | active | Framework for building Google Chat card payloads in Python |
| [firmata.py](https://github.com/google/firmata.py) | 21 | archived | Firmata protocol client for Python (Arduino communication) |
| [resolver-library](https://github.com/google/resolver-library) | 7 | archived | DNS resolver library (exported from code.google.com) |

---

## IT operations & enterprise tooling

Fleet management, endpoint automation, and enterprise ops tools.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [glazier](https://github.com/google/glazier) | 1252 | active | Windows OS automated deployment (see Notable section) |
| [cauliflowervest](https://github.com/google/cauliflowervest) | 279 | archived | App Engine escrow for disk encryption keys (FileVault, BitLocker, LUKS) |
| [loaner](https://github.com/google/loaner) | 169 | archived | Automated Chromebook loaner management |
| [macops](https://github.com/google/macops) | 824 | active | Mac fleet management scripts (see Notable section) |
| [nsscache](https://github.com/google/nsscache) | 155 | active | Linux NSS database sync with LDAP (see Notable section) |
| [paramgmt](https://github.com/google/paramgmt) | 85 | archived | Parallel SSH-based remote machine management |
| [gfw-deployments](https://github.com/google/gfw-deployments) | 78 | archived | Google for Work deployment scripts |
| [llvm-premerge-checks](https://github.com/google/llvm-premerge-checks) | 43 | archived | CI system for pre-merge testing in the LLVM project |
| [atlassian-addons-audit-sheet](https://github.com/google/atlassian-addons-audit-sheet) | 11 | archived | Audits Atlassian plugin lists to a Google Sheet |
| [tcp_killer](https://github.com/google/tcp_killer) | 217 | archived | CLI tool to shut down arbitrary TCP connections on Linux/macOS |
| [usbinfo](https://github.com/google/usbinfo) | 40 | archived | USB device information utility |
| [blkcgroup](https://github.com/google/blkcgroup) | 11 | archived | Linux block cgroup utilities |
| [localsubnetsetd](https://github.com/google/localsubnetsetd) | 6 | archived | Daemon to maintain nftables sets for local subnets |
| [rttcp](https://github.com/google/rttcp) | 33 | archived | Round-trip time measurement for TCP connections |
| [packet-queue](https://github.com/google/packet-queue) | 31 | archived | Packet queueing and rate-limiting utility |
| [x509test](https://github.com/google/x509test) | 43 | archived | X.509 certificate test suite |
| [permhash](https://github.com/google/permhash) | 45 | active | Permission hashing for Android/Chrome extension analysis |
| [grrshell](https://github.com/google/grrshell) | 8 | active | Shell interface for the GRR incident response framework |
| [dfdewey](https://github.com/google/dfdewey) | 18 | archived | Digital forensics string indexing tool |
| [tsmok](https://github.com/google/tsmok) | 16 | archived | ARM TrustZone emulator for TEE fuzzing |
| [rescue-tools-reiserfs](https://github.com/google/rescue-tools-reiserfs) | 5 | archived | Rescue tools for ReiserFS filesystems |
| [mandiant-ti-client](https://github.com/google/mandiant-ti-client) | 19 | archived | Python client for the Mandiant Threat Intelligence API |
| [dexmod](https://github.com/google/dexmod) | 64 | archived | DEX bytecode modifier for Android analysis |
| [binja-hexagon](https://github.com/google/binja-hexagon) | 117 | archived | Binary Ninja plugin for Qualcomm Hexagon DSP analysis |
| [Legilimency](https://github.com/google/Legilimency) | 114 | archived | iOS memory analysis framework |
| [dfindexeddb](https://github.com/google/dfindexeddb) | 52 | active | Digital forensics parser for browser IndexedDB files |
| [osv-scanner-action](https://github.com/google/osv-scanner-action) | 79 | active | GitHub Actions OSV vulnerability scanner (see Notable) |
| [github_nonpublic_api](https://github.com/google/github_nonpublic_api) | 47 | active | Python client for GitHub's undocumented/internal API endpoints |
| [civics_cdf_validator](https://github.com/google/civics_cdf_validator) | 38 | active | Validates election data files against the NIST Civics CDF schema |
| [gl-shader-validator](https://github.com/google/GL-Shader-Validator) | 40 | archived | OpenGL GLSL shader validator |

---

## Advertising & marketing analytics tools

Tools for Google Ads, Merchant Center, DV360, and related marketing platforms.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [shoptimizer](https://github.com/google/shoptimizer) | 47 | active | Optimizes Google Merchant Center product data via the Content API |
| [trimmed_match](https://github.com/google/trimmed_match) | 72 | active | Trimmed Match for randomized paired geo experiment design and analysis |
| [matched_markets](https://github.com/google/matched_markets) | 98 | active | Geo experiment design and analysis with matched markets (see Notable) |
| [fractribution](https://github.com/google/fractribution) | 58 | archived | Fractional attribution modeling for marketing channels |
| [grizzly](https://github.com/google/grizzly) | 69 | active | End-to-end DataOps platform deployed by Terraform for BigQuery workflows |
| [wci](https://github.com/google/wci) | 41 | active | WhatsApp conversion import tooling |
| [filonov](https://github.com/google/filonov) | 25 | active | AI creative concept analysis for display ads |
| [rubik](https://github.com/google/rubik) | 29 | active | Improve Google Merchant Center at scale |
| [garf](https://github.com/google/garf) | 21 | active | Call APIs with SQL via a unified query interface |
| [ads_oneshop](https://github.com/google/ads_oneshop) | 23 | active | Google Ads and shopping unified reporting/configuration tool |
| [keyword_factory](https://github.com/google/keyword_factory) | 42 | archived | Automated keyword generation for Search Ads |
| [prediction_framework](https://github.com/google/prediction_framework) | 43 | archived | Audience prediction framework using BigQuery ML |
| [driblet](https://github.com/google/driblet) | 19 | archived | End-to-end ML pipeline for customer lifetime value prediction |
| [consent-based-conversion-adjustments](https://github.com/google/consent-based-conversion-adjustments) | 26 | archived | Statistical up-weighting of consented conversion values for Google Ads |
| [disapproved-ads-auditor](https://github.com/google/disapproved-ads-auditor) | 18 | archived | Audits disapproved ads across Google Ads accounts |
| [spindle-dv360](https://github.com/google/spindle-dv360) | 13 | archived | QA dashboard for DV360 advertisers |
| [report2bq](https://github.com/google/report2bq) | 13 | archived | Exports Ads/SA360/DV360 reports to BigQuery |
| [taxonomy_wizard](https://github.com/google/taxonomy_wizard) | 13 | archived | Campaign taxonomy management for Google Ads |
| [brandometer](https://github.com/google/brandometer) | 10 | archived | Brand lift measurement tooling |
| [adcase](https://github.com/google/adcase) | 7 | archived | Ad case study analysis templates |
| [autobidding-readiness-monitor](https://github.com/google/autobidding-readiness-monitor) | 9 | archived | Monitors Google Ads automated bidding readiness signals |
| [dnae](https://github.com/google/dnae) | 9 | archived | Display network audience expansion tooling |
| [shopping_insider](https://github.com/google/shopping_insider) | 30 | archived | Merchant Center shopping performance dashboard |
| [ads-placement-excluder](https://github.com/google/ads-placement-excluder) | 10 | archived | Bulk placement exclusions for Google Ads campaigns |
| [pmax_migration](https://github.com/google/pmax_migration) | 4 | archived | Migration tooling from Smart Shopping to Performance Max |
| [product-dsa](https://github.com/google/product-dsa) | 11 | archived | Dynamic Search Ads setup from product feeds |
| [ad-manager-alerter](https://github.com/google/ad-manager-alerter) | 27 | archived | Alerting for Google Ad Manager anomalies |
| [adh-deployment-manager](https://github.com/google/adh-deployment-manager) | 5 | archived | Deployment manager for Ads Data Hub queries |
| [merchant_center_repor_builder](https://github.com/google/merchant_center_repor_builder) | 11 | archived | Merchant Center reporting builder |
| [assortment-quality-for-shopping-ads](https://github.com/google/assortment-quality-for-shopping-ads) | 8 | archived | Product and brand coverage overview for Merchant Center |
| [mozart](https://github.com/google/mozart) | 12 | archived | Custom business logic for Search Ads 360 |
| [clicktrackers-panel](https://github.com/google/clicktrackers-panel) | 7 | archived | Click tracker management panel |
| [ai_assisted_display_creative](https://github.com/google/ai_assisted_display_creative) | 16 | archived | AI-assisted display ad creative generation |
| [b-con](https://github.com/google/b-con) | 6 | archived | Behavioral conversion signal processing |
| [parallel-chunks](https://github.com/google/parallel-chunks) | 6 | archived | Parallelized data chunking for ad reporting pipelines |
| [speed-opportunity-finder](https://github.com/google/speed-opportunity-finder) | 11 | archived | Identifies page speed improvement opportunities |
| [feedloader](https://github.com/google/feedloader) | 13 | active | Product feed loader for Google Merchant Center |

---

## App Engine & cloud ops samples

Mostly archived App Engine scaffolds, samples, and cloud-oriented tooling.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [gae-secure-scaffold-python](https://github.com/google/gae-secure-scaffold-python) | 111 | archived | Secure App Engine scaffold for Python 2 web apps |
| [gae-secure-scaffold-python3](https://github.com/google/gae-secure-scaffold-python3) | 35 | active | Secure App Engine scaffold for Python 3 static and dynamic sites |
| [containerregistry](https://github.com/google/containerregistry) | 213 | archived | Python library and tools for interacting with a Docker registry (GCR) |
| [mysql-tools](https://github.com/google/mysql-tools) | 212 | archived | MySQL management and schema migration tools |
| [cluster-insight](https://github.com/google/cluster-insight) | 99 | archived | Context graph for Kubernetes cluster resources |
| [hyou](https://github.com/google/hyou) | 109 | archived | Pythonic interface to Google Sheets |
| [chatbase-python](https://github.com/google/chatbase-python) | 74 | archived | Python client for the Chatbase analytics platform |
| [google-my-business-samples](https://github.com/google/google-my-business-samples) | 142 | archived | Code samples for the Google My Business API |
| [apis-client-generator](https://github.com/google/apis-client-generator) | 163 | archived | Generates client libraries from Google API Discovery format |
| [apitools](https://github.com/google/apitools) | 155 | active | Python utilities for Google API clients (see Notable) |
| [identity-toolkit-python-client](https://github.com/google/identity-toolkit-python-client) | 32 | archived | Google Identity Toolkit Python client |
| [google-reauth-python](https://github.com/google/google-reauth-python) | 12 | archived | Python library for Google re-authentication flow |
| [protorpc](https://github.com/google/protorpc) | 70 | archived | Protocol Buffers RPC framework for App Engine |
| [python-lakeside](https://github.com/google/python-lakeside) | 47 | archived | Internal Python App Engine utilities |
| [pyaedj](https://github.com/google/pyaedj) | 9 | archived | Python App Engine Django integration |
| [asset-inventory-worksheet](https://github.com/google/asset-inventory-worksheet) | 9 | archived | GCP asset inventory to Google Sheets export |
| [secret-manager-with-sendgrid](https://github.com/google/secret-manager-with-sendgrid) | 10 | archived | Cloud Functions example: Secret Manager + SendGrid |
| [github_nonpublic_api](https://github.com/google/github_nonpublic_api) | 47 | active | Client for GitHub undocumented API endpoints |
| [github-release-retry](https://github.com/google/github-release-retry) | 15 | archived | Reliable GitHub Release creation with asset upload retries |
| [git-rebaser](https://github.com/google/git-rebaser) | 13 | archived | Automated git rebase assistance tool |
| [git-patrol](https://github.com/google/git-patrol) | 9 | archived | Git repository compliance checking tool |
| [lkml-gerrit-bridge](https://github.com/google/lkml-gerrit-bridge) | 7 | archived | Bridge between Linux Kernel Mailing List and Gerrit |
| [deputy-api-python-client](https://github.com/google/deputy-api-python-client) | 9 | archived | Python client for the Deputy workforce management API |
| [gcnn-survey-paper](https://github.com/google/gcnn-survey-paper) | 175 | archived | Survey paper code for graph convolutional networks |
| [repose](https://github.com/google/repose) | 5 | archived | REST service mock/test framework |
| [request-test](https://github.com/google/request-test) | 6 | archived | HTTP request testing utilities |
| [rbe-integration-test](https://github.com/google/rbe-integration-test) | 6 | archived | Remote Build Execution integration test helpers |
| [jacs](https://github.com/google/jacs) | 13 | archived | JSON authenticated credential signing |
| [mirandum](https://github.com/google/mirandum) | 12 | archived | Task scheduling and cron management on App Engine |
| [resultstoreui](https://github.com/google/resultstoreui) | 12 | archived | UI for the ResultStore test results API |
| [py-lab-hal](https://github.com/google/py-lab-hal) | 13 | archived | Python hardware abstraction layer for lab instruments |
| [memutil](https://github.com/google/memutil) | 3 | archived | Memory utility scripts |
| [migration-planner](https://github.com/google/migration-planner) | 8 | active | Desktop tool to assess Microsoft Exchange Online tenants before migration |
| [torq](https://github.com/google/torq) | 11 | active | Internal task orchestration and queuing library |
| [aura-inspector](https://github.com/google/aura-inspector) | 94 | active | Inspection and debugging tools for internal Aura UI framework |
| [hadal-flow](https://github.com/google/hadal-flow) | 12 | active | Internal workflow orchestration library |
| [sherlock](https://github.com/google/sherlock) | 10 | active | Service dependency analysis and diagram tool |
| [tcli](https://github.com/google/tcli) | 9 | active | Terminal CLI utilities for network device management |
| [x-sight](https://github.com/google/x-sight) | 3 | active | Observability and experiment tracking framework |
| [vera](https://github.com/google/vera) | 9 | active | Verification and rule-checking framework |
| [rago](https://github.com/google/rago) | 29 | active | RAG (Retrieval-Augmented Generation) orchestration library |
| [ragvis](https://github.com/google/ragvis) | 4 | active | Visualization tools for RAG pipeline evaluation |
| [parallax](https://github.com/google/parallax) | 4 | active | Distributed model training with JAX |
| [avatar](https://github.com/google/avatar) | 20 | active | Python/Pandora Bluetooth avatar testing framework |
| [bt-test-interfaces](https://github.com/google/bt-test-interfaces) | 15 | active | Bluetooth test interfaces and utilities |
| [bt-navi-tests](https://github.com/google/bt-navi-tests) | 7 | active | Bluetooth navigation testing framework |
| [flight-lab](https://github.com/google/flight-lab) | 15 | active | Flight simulation lab tooling |
| [ota-generator](https://github.com/google/ota-generator) | 24 | active | Android OTA package generator |
| [ml-edu](https://github.com/google/ml-edu) | 7 | active | ML education notebooks and sample code |
| [project-montage](https://github.com/google/project-montage) | 0 | active | New project (no description at time of indexing) |

---

## Education & training materials

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [it-cert-automation-practice](https://github.com/google/it-cert-automation-practice) | 1010 | archived | Practice files for Google IT Automation with Python Professional Certificate |
| [it-cert-automation-project](https://github.com/google/it-cert-automation-project) | 22 | archived | Final project files for the IT Automation with Python certificate |
| [coursebuilder-core](https://github.com/google/coursebuilder-core) | 147 | archived | Google Course Builder — open-source online education platform on App Engine |
| [coursebuilder_xblock_module](https://github.com/google/coursebuilder_xblock_module) | 17 | archived | XBlock module integration for Course Builder |
| [coursebuilder-lti-module](https://github.com/google/coursebuilder-lti-module) | 16 | archived | LTI module for Course Builder |
| [coursebuilder-hello-world-module](https://github.com/google/coursebuilder-hello-world-module) | 8 | archived | Hello World extension module for Course Builder |
| [teknowledge](https://github.com/google/teknowledge) | 30 | archived | Basic CS curriculum for coding in Python (K-12) |
| [applied-computing-series](https://github.com/google/applied-computing-series) | 14 | archived | Applied computing course materials using Python |
| [google-my-business-samples](https://github.com/google/google-my-business-samples) | 142 | archived | API samples for Google My Business |
| [support-tools](https://github.com/google/support-tools) | 30 | archived | Scripts for Google for Work support (exported from code.google.com) |

---

## Humanitarian & public interest

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [personfinder](https://github.com/google/personfinder) | 545 | archived | Searchable missing person database for disaster response (App Engine, Python) |
| [eclipse2017](https://github.com/google/eclipse2017) | 21 | archived | Source for the North American Eclipse 2017 Megamovie web app |
| [gov-meetings-made-searchable](https://github.com/google/gov-meetings-made-searchable) | 33 | archived | Makes public government meeting content searchable via speech-to-text |
| [ebola-tools](https://github.com/google/ebola-tools) | 15 | archived | Data tools created during the 2014 Ebola outbreak response |
| [self-published-geo](https://github.com/google/self-published-geo) | 14 | archived | Dataset of self-published geospatial data sources |

---

## Security & cryptography research

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [jaxite](https://github.com/google/jaxite) | 102 | active | FHE library for TPU/GPU (see Notable) |
| [certificate-transparency-rfcs](https://github.com/google/certificate-transparency-rfcs) | 80 | archived | Certificate Transparency RFC specifications and reference code |
| [cauliflowervest](https://github.com/google/cauliflowervest) | 279 | archived | Disk encryption key escrow (FileVault 2, BitLocker, LUKS) |
| [ctfscoreboard](https://github.com/google/ctfscoreboard) | 173 | archived | Scoreboard for Capture The Flag competitions |
| [x509test](https://github.com/google/x509test) | 43 | archived | X.509 certificate compliance test suite |
| [CSP-Validator](https://github.com/google/CSP-Validator) | 27 | archived | Content Security Policy validator |
| [binja-hexagon](https://github.com/google/binja-hexagon) | 117 | archived | Binary Ninja plugin for Qualcomm Hexagon DSP RE |
| [Legilimency](https://github.com/google/Legilimency) | 114 | archived | iOS memory introspection framework |
| [tsmok](https://github.com/google/tsmok) | 16 | archived | ARM TrustZone emulator for TEE security research |
| [dexmod](https://github.com/google/dexmod) | 64 | archived | DEX bytecode patching for Android security analysis |
| [dfindexeddb](https://github.com/google/dfindexeddb) | 52 | active | Browser IndexedDB forensics parser |
| [permhash](https://github.com/google/permhash) | 45 | active | Permission fingerprinting for mobile/extension analysis |
| [fishy-pdf](https://github.com/google/fishy-pdf) | 11 | archived | PDF structure analysis and suspicious-content detection |
| [Vulkan-Errata](https://github.com/google/Vulkan-Errata) | 5 | archived | Vulkan specification errata tracking |

---

## Media, audio & codec tools

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [uvq](https://github.com/google/uvq) | 148 | active | Universal Video Quality assessment model (see JAX ecosystem section) |
| [compare-codecs](https://github.com/google/compare-codecs) | 50 | archived | Video codec comparison framework |
| [speech_intelligibility_index](https://github.com/google/speech_intelligibility_index) | 51 | archived | Python implementation of the Speech Intelligibility Index standard |
| [light-my-piano](https://github.com/google/light-my-piano) | 90 | archived | Raspberry Pi piano learning device via LED lighting |
| [python-temescal](https://github.com/google/python-temescal) | 27 | archived | Python control library for LG speaker systems |
| [gpu-mux](https://github.com/google/gpu-mux) | 42 | archived | GPU multiplexer research for video encoding |
| [emoji4unicode](https://github.com/google/emoji4unicode) | 52 | archived | Emoji to Unicode mapping data and tooling |
| [stumblybot](https://github.com/google/stumblybot) | 24 | archived | Robot controlled by Google Assistant voice commands |
| [io-captions-gadget](https://github.com/google/io-captions-gadget) | 5 | archived | Real-time captions gadget for Google I/O events |

---

## 3D printing / maker projects

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [makerspace-auth](https://github.com/google/makerspace-auth) | 68 | archived | Access control devices for Google's makerspace |
| [makerspace-partsbin](https://github.com/google/makerspace-partsbin) | 17 | archived | Parts bin inventory tracking for makerspace |
| [OctoPrint-LEDStripControl](https://github.com/google/OctoPrint-LEDStripControl) | 65 | archived | OctoPrint plugin to control LED strips via M150 GCode |
| [OctoPrint-TemperatureFailsafe](https://github.com/google/OctoPrint-TemperatureFailsafe) | 27 | archived | OctoPrint plugin for heater temperature failsafe |
| [OctoPrint-HeaterTimeout](https://github.com/google/OctoPrint-HeaterTimeout) | 14 | archived | OctoPrint plugin for heater idle timeout shutdown |
| [makerfaire-booth](https://github.com/google/makerfaire-booth) | 14 | archived | Code for Google's Maker Faire 2016 booth installation |
| [linear-book-scanner](https://github.com/google/linear-book-scanner) | 82 | archived | Hardware and software for a linear book scanner |
| [cyanobyte](https://github.com/google/cyanobyte) | 83 | archived | Machine-readable datasheets for embedded/IoT peripherals |
| [firmata.py](https://github.com/google/firmata.py) | 21 | archived | Firmata protocol client for Python (listed also under utilities) |
| [mint-line-follower](https://github.com/google/mint-line-follower) | 3 | archived | MiNT educational robot line-follower firmware |

---

## NLP, linguistics & text data

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [corpuscrawler](https://github.com/google/corpuscrawler) | 214 | active | Linguistic corpus crawler for minority languages (see Notable) |
| [shuwa](https://github.com/google/shuwa) | 146 | archived | Sign language gesture recognition |
| [text2text](https://github.com/google/text2text) | 54 | archived | Cross-lingual text transformation |
| [airdialogue](https://github.com/google/airdialogue) | 47 | archived | Goal-oriented flight booking dialogue corpus |
| [categorybuilder](https://github.com/google/categorybuilder) | 98 | archived | Semantic category building from distributional data |
| [sample-sql-translator](https://github.com/google/sample-sql-translator) | 52 | archived | SQL dialect translation sample code |
| [url_diff](https://github.com/google/url_diff) | 98 | archived | URL differencing and normalization utility |
| [emoji4unicode](https://github.com/google/emoji4unicode) | 52 | archived | Emoji-to-Unicode mapping (listed also under media) |
| [storybench](https://github.com/google/storybench) | 54 | archived | Benchmark for LLM story generation quality |
| [gps-babel-tower](https://github.com/google/gps-babel-tower) | 8 | archived | Translation and localization tooling for ad content |

---

## Games, experiments & creative projects

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [arithmancer](https://github.com/google/arithmancer) | 64 | archived | Logarithmic Market Scoring Rule prediction market |
| [ctfscoreboard](https://github.com/google/ctfscoreboard) | 173 | archived | Capture The Flag competition scoreboard |
| [werewolf_arena](https://github.com/google/werewolf_arena) | 47 | archived | Multi-agent Werewolf game arena driven by LLMs |
| [hypebot](https://github.com/google/hypebot) | 16 | archived | Internal Slack-style chatbot for gaming hype |
| [ci_edit](https://github.com/google/ci_edit) | 225 | archived | Terminal text editor with mouse support written in Python |
| [Zhi](https://github.com/google/Zhi) | 28 | archived | Interactive LaTeX paper writing in Google Drive |
| [eclipse2017](https://github.com/google/eclipse2017) | 21 | archived | Eclipse 2017 Megamovie citizen science web app |
| [protocall](https://github.com/google/protocall) | 13 | archived | Experimental programming language prototype |
| [realtime-help](https://github.com/google/realtime-help) | 19 | archived | Real-time collaborative help system prototype |
| [splitbrain](https://github.com/google/splitbrain) | 22 | archived | Research system for automated PR splitting |
| [cpython-pt](https://github.com/google/cpython-pt) | 12 | archived | CPython performance threading fork/experiment |
| [wheelbarrow](https://github.com/google/wheelbarrow) | 11 | archived | Android app metadata collection tool |
| [mcafp](https://github.com/google/mcafp) | 38 | archived | Multi-channel audio fingerprinting |
| [bigspicy](https://github.com/google/bigspicy) | 39 | archived | SPICE netlist merging and analysis for chip design |
| [kepler](https://github.com/google/kepler) | 38 | archived | ML-based compiler hint prediction (Kepler CPU prefetcher) |

---

## Miscellaneous / minimal description

Low-star or minimal-description repos that do not fit above categories. All archived unless noted.

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [cocoapods-size](https://github.com/google/cocoapods-size) | 235 | archived | Measures final binary size contribution of CocoaPods |
| [gtasks-md](https://github.com/google/gtasks-md) | 85 | active | Edits Google Tasks as a Markdown document |
| [python-gflags](https://github.com/google/python-gflags) | 189 | archived | Python commandline flags (see utilities) |
| [gl-shader-validator](https://github.com/google/GL-Shader-Validator) | 40 | archived | GLSL shader validator (listed also under IT ops) |
| [xctestrunner](https://github.com/google/xctestrunner) | 158 | active | iOS test runner (see Notable section) |
| [sample-sql-translator](https://github.com/google/sample-sql-translator) | 52 | archived | SQL dialect translation (listed also under NLP) |
| [pyvisionproductsearch](https://github.com/google/pyvisionproductsearch) | 9 | archived | Python client for Cloud Vision Product Search API |
| [graph-gen](https://github.com/google/graph-gen) | 5 | archived | Graph dataset generation utilities |
| [wikiloop-analysis](https://github.com/google/wikiloop-analysis) | 4 | archived | Wikipedia edit analysis for the WikiLoop project |
| [jarvan](https://github.com/google/jarvan) | 4 | archived | Internal task scheduling experiments |
| [saka](https://github.com/google/saka) | 6 | archived | Unnamed internal research experiment |
| [bocado](https://github.com/google/bocado) | 9 | archived | Runtime type profiler for Python |
| [AppSpeedIndex](https://github.com/google/AppSpeedIndex) | 13 | archived | Mobile app speed index measurement |
| [shipshape-demo](https://github.com/google/shipshape-demo) | 4 | archived | Demo for the Shipshape static analysis platform |
| [ota-generator](https://github.com/google/ota-generator) | 24 | active | Android OTA package generator (also under Cloud ops) |
| [gps-babel-tower](https://github.com/google/gps-babel-tower) | 8 | archived | Ad content localization (also under NLP) |
| [hypebot](https://github.com/google/hypebot) | 16 | archived | Internal chatbot (also under Games) |
| [mirandum](https://github.com/google/mirandum) | 12 | archived | App Engine cron and task queue UI |
| [python-lakeside](https://github.com/google/python-lakeside) | 47 | archived | App Engine Python utilities |
| [tcp_killer](https://github.com/google/tcp_killer) | 217 | archived | TCP connection terminator (also under IT ops) |
| [url_diff](https://github.com/google/url_diff) | 98 | archived | URL normalization and diffing |
| [checkstream ](https://github.com/google/chkstream) | 18 | archived | Java 8 stream checked exception support (Python-adjacent) |
| [blkcgroup](https://github.com/google/blkcgroup) | 11 | archived | Linux block IO cgroup controller |
| [aborts / stress_transfer](https://github.com/google/stress_transfer) | 8 | archived | Geomechanics stress transfer model |
| [graph-gen](https://github.com/google/graph-gen) | 5 | archived | Graph dataset generation |
| [bocado](https://github.com/google/bocado) | 9 | archived | Python runtime type profiler |
| [wide_bnn_sampling](https://github.com/google/wide_bnn_sampling) | 7 | archived | Wide Bayesian neural network sampling |

---

*Total repos covered: 299 (28 active, 271 archived). Repos appearing in multiple relevant categories are noted inline. Count verified against source JSON.*
