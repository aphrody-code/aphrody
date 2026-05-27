# Google · AI / Machine Learning

Google's open-source AI/ML Python ecosystem spans foundation model weights, agent frameworks, JAX-based training infrastructure, NLP tooling, RL environments, and domain-specific research code. The repositories range from actively maintained production libraries to archived research artifacts accompanying published papers.

> Part of [`docs/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 80 repos (55 active / 25 archived).

## Foundation Models & LLMs

### [langextract](https://github.com/google/langextract)
**★ 36514 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `gemini` `gemini-api` `information-extration` `large-language-models` `llm` `nlp` `python` `structured-data`

LangExtract is a Python library that uses LLMs to extract structured information from unstructured text documents based on user-defined instructions. It maps every extraction to its exact source location in the input text, enabling visual highlighting for traceability and verification. The library supports chunking and parallel processing for long documents, generates interactive HTML visualization reports, and works with Gemini, OpenAI, and local Ollama models without requiring fine-tuning.

### [gemma_pytorch](https://github.com/google/gemma_pytorch)
**★ 5674 · `active` · pushed 2025-05 · Apache-2.0**
Topics: `gemma` `google` `pytorch`

Official PyTorch implementation of Google's Gemma open model family, covering Gemma 1, Gemma 2, and Gemma 3 (1B–27B parameters, including multimodal variants). The repository provides model definitions and inference code for CPU, GPU, and TPU via PyTorch/XLA. Checkpoints are distributed through Kaggle and Hugging Face Hub.

### [langfun](https://github.com/google/langfun)
**★ 911 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `framework` `llms` `nlp`

langfun provides an object-oriented programming model for LLMs, treating language model interactions as composable Python objects. It enables structured prompting, schema-enforced generation, and modular pipeline construction. The library is used internally at Google Research for rapid prototyping of LLM-based workflows.

### [trax](https://github.com/google/trax)
**★ 8304 · `archived` · pushed 2025-09 · Apache-2.0**
Topics: `deep-learning` `deep-reinforcement-learning` `jax` `machine-learning` `numpy` `reinforcement-learning` `transformer`

Trax was a deep learning library emphasizing readable code and speed, implementing Transformer variants and sequence models in JAX/NumPy. It served as the research vehicle for several Google Brain papers on efficient Transformers. The repository is archived; its successor work moved to other JAX-based frameworks.

### [seq2seq](https://github.com/google/seq2seq)
**★ 5627 · `archived` · pushed 2020-10 · Apache-2.0**

General-purpose TensorFlow encoder-decoder framework from 2017. Implemented sequence-to-sequence models for machine translation and summarization, with configurable attention mechanisms and beam search decoding. Now archived; superseded by subsequent Transformer-based approaches.

---

## Agents & Skills

### [adk-python](https://github.com/google/adk-python)
**★ 19777 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `agent` `agentic` `agents-sdk` `ai-agents` `aiagentframework` `genai` `llm` `multi-agent` `multi-agent-systems`

Agent Development Kit (ADK) 2.0 is an open-source, code-first Python framework for building, evaluating, and deploying AI agents. Version 2.0 introduces a graph-based Workflow Runtime for composing deterministic execution flows with support for routing, fan-out/fan-in, loops, retry, state management, and human-in-the-loop. The Task API provides structured agent-to-agent delegation with multi-turn and single-turn modes. Requires Python 3.11+; install via `pip install google-adk`.

### [adk-samples](https://github.com/google/adk-samples)
**★ 9366 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `adk` `agent-samples` `agents`

Collection of ready-to-use sample agents built on ADK, covering Python, TypeScript, Go, Java, Kotlin, and Android. Examples range from simple conversational bots to complex multi-agent workflows, intended to accelerate development with concrete, runnable reference implementations.

### [skills](https://github.com/google/skills)
**★ 10176 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `google` `googlecloud` `skills`

Curated repository of Agent Skills for Google products and technologies, including Gemini API, Vertex AI Managed Agents, AlloyDB, BigQuery, Cloud Run, Cloud SQL, Firebase, and GKE. Skills are installable via the `skills.sh` platform (`npx skills add google/skills`) and are structured as Markdown-based capability definitions for AI agents.

### [adk-python-community](https://github.com/google/adk-python-community)
**★ 175 · `active` · pushed 2026-05 · Apache-2.0**

Community extensions, plugins, and example contributions for the `adk-python` framework. Complements the official `adk-samples` repo with community-driven patterns and integrations.

---

## Post-Training & Fine-Tuning

### [tunix](https://github.com/google/tunix)
**★ 2288 · `active` · pushed 2026-05 · Apache-2.0**

Tunix (Tune-in-JAX) is a lightweight JAX-based library for LLM post-training, supporting supervised fine-tuning (full weights and PEFT/LoRA), DPO, ORPO, PPO, and GRPO on TPUs. It integrates with Flax NNX, Optax, and Orbax, and connects to vLLM and SGLang-JAX for rollout. Designed to sit between core JAX utilities and production model frameworks like MaxText.

### [seqio](https://github.com/google/seqio)
**★ 593 · `active` · pushed 2026-05 · Apache-2.0**

Task-based dataset library for sequence models, providing preprocessing pipelines, vocabularies, and evaluation utilities designed for large-scale text tasks. Used extensively with T5, T5X, and related Google Research models as a standard data interface layer.

---

## ML Frameworks & Libraries

### [paxml](https://github.com/google/paxml)
**★ 551 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `c4` `gpt` `jax` `large-language-models` `llm` `model-flops` `parallelism`

Pax is a JAX-based ML framework for training large-scale models, providing advanced and fully configurable experimentation and parallelization. It demonstrated industry-leading model FLOP utilization rates and supports data, model, and pipeline parallelism on TPU pods.

### [orbax](https://github.com/google/orbax)
**★ 508 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `checkpoint` `flax` `jax`

Orbax provides checkpointing and persistence utilities for the JAX ecosystem, including async checkpointing, distributed save/restore, and multi-host support. It is the standard checkpointing solution for Flax and Pax-based models.

### [jaxopt](https://github.com/google/jaxopt)
**★ 1040 · `active` · pushed 2025-12 · Apache-2.0**
Topics: `bi-level` `deep-learning` `differentiable-programming` `jax` `optimization`

JAX-native optimization library providing hardware-accelerated, batchable, and differentiable first- and second-order optimizers. Covers proximal gradient, projected gradient, implicit differentiation, and bi-level optimization, with support for stochastic variants and GPU/TPU execution.

### [torchax](https://github.com/google/torchax)
**★ 220 · `active` · pushed 2026-05 · Apache-2.0**

PyTorch frontend for JAX that allows authoring JAX programs using PyTorch syntax and mixing JAX/PyTorch operations in the same program. Enables running PyTorch-syntax models on any hardware JAX supports (CPU, GPU, TPU, Neuron) without full code rewrites.

### [jaxonnxruntime](https://github.com/google/jaxonnxruntime)
**★ 135 · `active` · pushed 2026-04 · Apache-2.0**

Tool chain for executing ONNX models using JAX as the backend. Enables JAX-based inference for models exported from PyTorch, TensorFlow, or other frameworks via the ONNX interchange format.

### [temporian](https://github.com/google/temporian)
**★ 713 · `active` · pushed 2025-10 · Apache-2.0**

Python library for preprocessing and feature engineering of temporal data for ML applications. Provides a DataFrame-like API optimized for time-series operations, with support for windowing, aggregation, joins over time, and efficient execution on CPU/GPU.

### [qkeras](https://github.com/google/qkeras)
**★ 582 · `active` · pushed 2026-02 · Apache-2.0**

Quantization deep learning library extending Keras with quantized layers and activation functions. Supports post-training quantization and quantization-aware training targeting deployment on edge devices and hardware accelerators.

### [aqt](https://github.com/google/aqt)
**★ 355 · `active` · pushed 2026-04 · Apache-2.0**

Accurate Quantized Training (AQT) provides JAX/Flax primitives for quantization-aware training with precise numerical control over weights and activations. Used for preparing large models for integer quantization on TPUs and edge hardware.

### [qwix](https://github.com/google/qwix)
**★ 115 · `active` · pushed 2026-05 · Apache-2.0**

JAX quantization library providing composable quantization transformations for model weights and activations. Designed to integrate with Flax NNX and the broader JAX training stack.

### [sequence-layers](https://github.com/google/sequence-layers)
**★ 61 · `active` · pushed 2026-05 · Apache-2.0**

Neural network layer API for sequence modeling that supports both layerwise execution (used during training) and stepwise execution (used during autoregressive sampling), eliminating the need for separate training and inference codepaths.

### [metrax](https://github.com/google/metrax)
**★ 58 · `active` · pushed 2026-04 · Apache-2.0**

JAX-native evaluation metrics library providing differentiable, hardware-accelerated implementations of common ML metrics. Designed for use with JAX training loops where metrics need to run inside JIT-compiled functions.

### [fedjax](https://github.com/google/fedjax)
**★ 271 · `active` · pushed 2026-04 · Apache-2.0**

FedJAX is a JAX-based library for federated learning simulations, emphasizing research ease-of-use. Provides implementations of federated optimization algorithms (FedAvg, Mime, etc.) and tools for constructing custom federated datasets.

---

## Hyperparameter Optimization & AutoML

### [vizier](https://github.com/google/vizier)
**★ 1646 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `bayesian-optimization` `blackbox-optimization` `hyperparameter-optimization` `hyperparameter-tuning` `machine-learning` `optimization` `vizier`

Open Source Vizier is a Python-based service for black-box and hyperparameter optimization, based on the internal Google Vizier service (one of the first hyperparameter tuning systems at scale). It provides a distributed client-server architecture, supports Bayesian optimization, evolutionary algorithms, and custom search algorithms, and exposes a gRPC API for integration with existing training pipelines.

---

## Datasets & Benchmarks

### [BIG-bench](https://github.com/google/BIG-bench)
**★ 3240 · `archived` · pushed 2024-07 · Apache-2.0**

Beyond the Imitation Game Benchmark (BIG-bench): a collaborative benchmark of 200+ diverse tasks for probing large language models, designed to measure capabilities that current models struggle with and to extrapolate future model behavior. Includes JSON tasks and programmatic tasks; has an associated BIG-bench Lite leaderboard of 24 representative tasks.

### [deepvariant](https://github.com/google/deepvariant)
**★ 3705 · `active` · pushed 2026-03 · BSD-3-Clause**
Topics: `bioinformatics` `deep-learning` `deepvariant` `dna` `genome` `genomics` `machine-learning` `tensorflow`

DeepVariant is a deep-learning-based variant caller for next-generation DNA sequencing data. It converts aligned reads (BAM/CRAM) into pileup image tensors, classifies them with a CNN, and outputs VCF/gVCF files. Supports Illumina, PacBio HiFi, Oxford Nanopore R10.4.1, Complete Genomics, and hybrid inputs; includes pangenome-aware calling modes.

### [youtube-8m](https://github.com/google/youtube-8m)
**★ 2376 · `active` · pushed 2021-10 · Apache-2.0**

Starter code and baselines for the YouTube-8M dataset: a large-scale labeled video dataset with 8 million YouTube video IDs and 4716 classes. Provides TensorFlow models for video-level and frame-level classification, including mixture-of-experts and LSTM-based approaches.

### [meridian](https://github.com/google/meridian)
**★ 1385 · `active` · pushed 2026-05 · Apache-2.0**

Meridian is a Bayesian Marketing Mix Modeling (MMM) framework for measuring marketing channel ROI and optimizing budget allocation. It handles geo-level and national-level data, supports calibration with experiments and prior information, and provides built-in reach/frequency analysis. Positioned as the successor to LightweightMMM.

### [tf-quant-finance](https://github.com/google/tf-quant-finance)
**★ 5363 · `active` · pushed 2026-02 · Apache-2.0**
Topics: `finance` `gpu-computing` `high-performance` `numerical-methods` `quantitative-finance` `tensorflow`

High-performance TensorFlow library for quantitative finance, providing GPU-accelerated implementations of numerical methods for option pricing, rate models, Black-Scholes, Monte Carlo simulation, and PDE solvers. Targets production deployment in financial institutions.

---

## ML for Systems & Specialized Domains

### [ml-compiler-opt](https://github.com/google/ml-compiler-opt)
**★ 778 · `active` · pushed 2026-05 · Apache-2.0**

MLGO framework for integrating machine learning into LLVM compiler optimizations, replacing hand-crafted heuristics with learned policies. Currently supports inlining-for-size and register-allocation-for-performance, trained via Policy Gradient and Evolution Strategies. Compiler components are upstream in LLVM; this repository contains the training infrastructure.

### [gematria](https://github.com/google/gematria)
**★ 94 · `active` · pushed 2026-02 · Apache-2.0**

Machine learning for machine code: tools for learning performance models of x86 basic blocks. Used to predict instruction throughput and latency for compiler cost modeling, enabling learned models to replace hand-tuned CPU performance tables.

### [scaaml](https://github.com/google/scaaml)
**★ 197 · `active` · pushed 2026-05 · Apache-2.0**

Side Channel Attacks Assisted with Machine Learning (SCAAML): a framework for applying deep learning to hardware side-channel attacks on cryptographic implementations. Provides attack pipelines, dataset tools, and pretrained models targeting AES and other ciphers.

### [deepconsensus](https://github.com/google/deepconsensus)
**★ 265 · `active` · pushed 2026-04 · BSD-3-Clause**

DeepConsensus uses gap-aware sequence Transformers to correct errors in PacBio Circular Consensus Sequencing (CCS) reads. It improves CCS read accuracy and yield, enabling more cost-effective long-read sequencing workflows.

---

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [model_search](https://github.com/google/model_search) | 3245 | archived | AutoML framework for neural architecture search |
| [lightweight_mmm](https://github.com/google/lightweight_mmm) | 1044 | archived | Bayesian Marketing Mix Modeling library (superseded by Meridian) |
| [uis-rnn](https://github.com/google/uis-rnn) | 1588 | archived | Unbounded Interleaved-State RNN for speaker diarization |
| [prettytensor](https://github.com/google/prettytensor) | 1227 | archived | Fluent network building API for TensorFlow |
| [active-learning](https://github.com/google/active-learning) | 1164 | archived | Active learning utilities for TensorFlow |
| [hypernerf](https://github.com/google/hypernerf) | 960 | archived | Higher-dimensional NeRF for topologically varying scenes |
| [objax](https://github.com/google/objax) | 775 | archived | Object-oriented JAX neural network library |
| [flaxformer](https://github.com/google/flaxformer) | 367 | archived | Flax-based Transformer implementations (T5, etc.) |
| [rax](https://github.com/google/rax) | 336 | active | Learning-to-rank library written in JAX |
| [lmeval](https://github.com/google/lmeval) | 236 | active | Language model evaluation utilities |
| [aistplusplus_api](https://github.com/google/aistplusplus_api) | 389 | archived | API for AIST++ dance dataset |
| [balloon-learning-environment](https://github.com/google/balloon-learning-environment) | 131 | archived | RL environment for stratospheric balloon navigation |
| [revisiting-self-supervised](https://github.com/google/revisiting-self-supervised) | 352 | archived | Self-supervised visual representation learning research code |
| [mentornet](https://github.com/google/mentornet) | 327 | archived | Data-driven curriculum learning for deep networks |
| [neural-logic-machines](https://github.com/google/neural-logic-machines) | 295 | archived | Neural Logic Machines for relational reasoning |
| [neural-light-transport](https://github.com/google/neural-light-transport) | 278 | archived | Neural light transport for relighting and view synthesis |
| [space](https://github.com/google/space) | 155 | archived | Unified ML lifecycle storage framework |
| [bayesnf](https://github.com/google/bayesnf) | 150 | archived | Bayesian Neural Field models for spatiotemporal datasets |
| [tree-math](https://github.com/google/tree-math) | 210 | archived | Mathematical operations for JAX pytrees |
| [spectral-density](https://github.com/google/spectral-density) | 125 | archived | Hessian spectral density estimation in TF and JAX |
| [attention-center](https://github.com/google/attention-center) | 121 | active | Visual attention center prediction model |
| [ARC-GEN](https://github.com/google/ARC-GEN) | 47 | active | Procedural benchmark generator for Abstraction and Reasoning Corpus |
| [rag-playground](https://github.com/google/rag-playground) | 49 | active | RAG experimentation and prototyping tools |
| [struct2tensor](https://github.com/google/struct2tensor) | 36 | active | Parsing and manipulating structured data inside TensorFlow |
| [vertex-ai-nas](https://github.com/google/vertex-ai-nas) | 30 | active | Neural architecture search via Vertex AI |
| [t5patches](https://github.com/google/t5patches) | 12 | archived | Fast targeted editing of T5X-based language models |
| [dspl](https://github.com/google/dspl) | 63 | archived | Schema and utilities for Google Dataset Publishing Language |
| [spiqa](https://github.com/google/spiqa) | 75 | archived | Multimodal QA dataset for scientific papers (NeurIPS 2024) |
| [feabench](https://github.com/google/feabench) | 13 | active | LLM multiphysics reasoning benchmark (MATH-AI @ NeurIPS 2024) |
| [wasserstein-dist](https://github.com/google/wasserstein-dist) | 74 | archived | TensorFlow implementation of Wasserstein/optimal-transport distance |
| [qhbm-library](https://github.com/google/qhbm-library) | 43 | archived | Quantum Hamiltonian-Based Models on TensorFlow Quantum |
| [cog](https://github.com/google/cog) | 44 | archived | COG visual question-answering dataset and model code |
| [jax-datetime](https://github.com/google/jax-datetime) | 12 | active | JAX-compatible datetime and timedelta types |
| [jax-recommenders](https://github.com/google/jax-recommenders) | 11 | archived | Recommendation model experiments in JAX |
| [prompt-encryption-sdk](https://github.com/google/prompt-encryption-sdk) | 10 | active | SDK for encrypting prompts sent to LLM APIs |
| [bespoke](https://github.com/google/bespoke) | 5 | active | Spaced repetition system specialized for language learning |
| [adk-conformance](https://github.com/google/adk-conformance) | 4 | active | Conformance tests for ADK implementations |
| [agamotto](https://github.com/google/agamotto) | 18 | archived | ML for insights from physical locations |
| [embedding-tests](https://github.com/google/embedding-tests) | 17 | archived | Embedding quality evaluation utilities |
| [timecast](https://github.com/google/timecast) | 16 | archived | Composable online learning library |
| [generativemloncloud](https://github.com/google/generativemloncloud) | 58 | archived | Generative ML on Cloud samples (2017-2018) |
| [simple-reinforcement-learning](https://github.com/google/simple-reinforcement-learning) | 58 | archived | Simple RL tutorial code |
| [project-OCEAN](https://github.com/google/project-OCEAN) | 56 | archived | Open-source ecosystem research datasets |
| [chiplets-cost-model](https://github.com/google/chiplets-cost-model) | 25 | archived | ML-based chiplet cost modeling |
| [cloud-function-edit-drive-permissions](https://github.com/google/cloud-function-edit-drive-permissions) | 15 | archived | Cloud Function for Drive permission management |
| [profiling-data-processing-model-isca23-ae](https://github.com/google/profiling-data-processing-model-isca23-ae) | 9 | archived | ISCA 2023 artifact: data processing profiling model |
| [ic-modeling-python](https://github.com/google/ic-modeling-python) | 1 | archived | IC design modeling utilities |
