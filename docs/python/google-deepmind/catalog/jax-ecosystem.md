# Google DeepMind · JAX Ecosystem

Libraries, utilities, and research tools built on JAX, covering neural network construction, optimization, probabilistic inference, graph learning, and scientific computing.

> Part of [`docs/python/google-deepmind/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 38 repos (32 active / 6 archived).

## Core neural network libraries

### [dm-haiku](https://github.com/google-deepmind/dm-haiku)
**★ 3233 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `deep-learning` `deep-neural-networks` `jax` `machine-learning` `neural-networks`  
JAX-based neural network library offering an object-oriented module system via a functional-by-default transform (`hk.transform`). Widely used across DeepMind research as the primary JAX network construction library; used in GraphCast, Gemma, and most JAX-based DeepMind systems.

### [penzai](https://github.com/google-deepmind/penzai)
**★ 1884 · `active` · pushed 2025-06 · Apache-2.0**  
Topics: `fine-tuning` `interpretability` `jax` `neural-networks` `visualization`  
JAX research toolkit for building, editing, and visualizing neural networks as functional pytree structures. Particularly designed for interpretability work and for interactively modifying pre-trained models.

### [distrax](https://github.com/google-deepmind/distrax)
**★ 634 · `active` · pushed 2026-05 · Apache-2.0**  
JAX re-implementation of TensorFlow Probability (distributions and bijectors). Provides probability distributions, normalizing flows, and sampling utilities that integrate natively with JAX transforms (vmap, jit, grad).

### [enn](https://github.com/google-deepmind/enn)
**★ 315 · `active` · pushed 2026-02 · Apache-2.0**  
Epistemic Neural Networks: a library for representing and training networks that explicitly model uncertainty over function space, rather than just over weights.

## Optimization and training utilities

### [optax](https://github.com/google-deepmind/optax)
**★ 2269 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `machine-learning` `optimization`  
Gradient processing and optimization library for JAX. Provides composable gradient transformations (Adam, SGD, RMSProp, Lion, etc.), learning rate schedules, gradient clipping, and loss functions. The standard optimizer library for JAX research.

### [kfac-jax](https://github.com/google-deepmind/kfac-jax)
**★ 324 · `active` · pushed 2026-05 · Apache-2.0**  
Second-order optimization via Kronecker-Factored Approximate Curvature (K-FAC) in JAX. Provides curvature estimation and natural gradient updates for large neural networks.

### [jmp](https://github.com/google-deepmind/jmp)
**★ 213 · `active` · pushed 2025-01 · Apache-2.0**  
JMP is a Mixed Precision library for JAX, providing loss scaling and policy management for bfloat16/float16 training with minimal boilerplate.

### [dks](https://github.com/google-deepmind/dks)
**★ 79 · `active` · pushed 2025-07 · Apache-2.0**  
Multi-framework implementation of Deep Kernel Shaping and Tailored Activation Transformations. Methods that modify network architectures and initializations to enable training without batch normalization.

## Testing and verification

### [chex](https://github.com/google-deepmind/chex)
**★ 941 · `active` · pushed 2026-04 · Apache-2.0**  
JAX testing and verification library. Provides `assert_*` functions for shape checking, dtype validation, and numerical correctness, plus utilities for writing parameterized tests and comparing JAX variants (jit, vmap, pmap).

### [jax_verify](https://github.com/google-deepmind/jax_verify)
**★ 145 · `archived` · pushed 2023-08 · Apache-2.0**  
Neural network verification in JAX. Implements bound propagation methods (IBP, CROWN, FastLin) for certifying robustness properties of JAX networks.

### [jax_privacy](https://github.com/google-deepmind/jax_privacy)
**★ 170 · `active` · pushed 2026-05 · Apache-2.0**  
Differentially private machine learning in JAX. Implements DP-SGD with per-sample gradient computation, privacy accounting, and auditing utilities.

## Graph learning

### [jraph](https://github.com/google-deepmind/jraph)
**★ 1470 · `archived` · pushed 2024-03 · Apache-2.0**  
Topics: `deep-learning` `graph-neural-networks` `jax` `machine-learning`  
Graph Neural Network library in JAX using a lightweight GraphsTuple data structure. Archived in favour of downstream frameworks but remains widely cited as a reference implementation.

### [synjax](https://github.com/google-deepmind/synjax)
**★ 250 · `active` · pushed 2026-02 · Apache-2.0**  
Structured prediction library in JAX implementing dynamic programming algorithms (CKY, inside-outside, Viterbi) over latent graphical structures for NLP and structured neural models.

## Training frameworks and infrastructure

### [jaxline](https://github.com/google-deepmind/jaxline)
**★ 166 · `active` · pushed 2023-12 · Apache-2.0**  
Lightweight training framework for JAX experiments. Provides experiment configuration, checkpointing, multi-host training setup, and evaluation loops with minimal abstraction overhead.

### [xmanager](https://github.com/google-deepmind/xmanager)
**★ 909 · `active` · pushed 2026-05 · Apache-2.0**  
Platform-agnostic ML experiment management. Supports launching jobs to local machines, Google Cloud, or cluster schedulers, with structured configuration and result tracking.

### [treescope](https://github.com/google-deepmind/treescope)
**★ 470 · `active` · pushed 2025-08 · Apache-2.0**  
Interactive HTML pretty-printer for JAX/NumPy arrays and pytrees in IPython notebooks. Enables interactive folding, inspection of large tensors, and in-notebook neural network visualization.

### [tree](https://github.com/google-deepmind/tree)
**★ 1022 · `active` · pushed 2026-03 · Apache-2.0**  
Library for working with nested Python data structures (pytrees). Provides map, reduce, flatten, and traversal operations analogous to `dm-tree` from older DeepMind research.

### [einshape](https://github.com/google-deepmind/einshape)
**★ 110 · `active` · pushed 2024-06 · Apache-2.0**  
Minimal reshape / rearrange utility using Einstein notation, compatible with JAX, NumPy, and TensorFlow arrays.

### [tf2jax](https://github.com/google-deepmind/tf2jax)
**★ 123 · `active` · pushed 2026-04 · Apache-2.0**  
Convert TensorFlow 2 functions and SavedModels to JAX functions, enabling migration of TF-trained models into a JAX computation graph without re-training.

## Probabilistic and generative models

### [annealed_flow_transport](https://github.com/google-deepmind/annealed_flow_transport)
**★ 53 · `archived` · pushed 2023-02 · Apache-2.0**  
JAX implementation of Annealed Flow Transport MCMC, combining normalizing flows with sequential Monte Carlo for approximating intractable posteriors.

### [flows_for_atomic_solids](https://github.com/google-deepmind/flows_for_atomic_solids)
**★ 53 · `archived` · pushed 2022-10 · Apache-2.0**  
Normalizing flow-based free energy estimation for atomic solid systems using JAX.

### [md4](https://github.com/google-deepmind/md4)
**★ 160 · `active` · pushed 2025-02 · Apache-2.0**  
Official JAX implementation of MD4 masked diffusion models for discrete sequence generation.

### [implicit_diffusion](https://github.com/google-deepmind/implicit_diffusion)
**★ 10 · `active` · pushed 2025-03 · Apache-2.0**  
JAX implementation of implicit diffusion training procedures.

### [conformal_training](https://github.com/google-deepmind/conformal_training)
**★ 131 · `archived` · pushed 2022-08 · Apache-2.0**  
JAX implementation of "Learning Optimal Conformal Classifiers" (ICLR 2022), integrating conformal calibration directly into the training objective.

### [uncertain_ground_truth](https://github.com/google-deepmind/uncertain_ground_truth)
**★ 681 · `archived` · pushed 2024-03 · Apache-2.0**  
Dermatology DDx dataset, JAX implementations of Monte Carlo conformal prediction, plausibility regions, and statistical annotation aggregation. Companion to TMLR 2023 paper.

## Scientific simulation in JAX

### [torax](https://github.com/google-deepmind/torax)
**★ 673 · `active` · pushed 2026-05 · Other**  
Tokamak transport simulation in JAX, implementing 1D PDE-based plasma transport models accelerated by JAX JIT. Used to generate training data for surrogate ML models and for RL-based plasma control research.

### [fusion_surrogates](https://github.com/google-deepmind/fusion_surrogates)
**★ 37 · `active` · pushed 2026-05 · Apache-2.0**  
Library of surrogate transport models for tokamak fusion, providing fast JAX-based approximations to physics simulators for use in control and RL applications.

### [jeo](https://github.com/google-deepmind/jeo)
**★ 160 · `active` · pushed 2025-11 · Apache-2.0**  
JAX model training library for Earth Observation. Handles multi-spectral satellite imagery, patch-based training, and integration with geospatial data formats.

### [geeflow](https://github.com/google-deepmind/geeflow)
**★ 115 · `active` · pushed 2025-11 · Apache-2.0**  
GeeFlow: generate and process large-scale geospatial datasets with Google Earth Engine, bridging GEE data pipelines to JAX training workflows.

### [xarray_jax](https://github.com/google-deepmind/xarray_jax)
**★ 42 · `active` · pushed 2026-04 · Apache-2.0**  
JAX backend for xarray, enabling labeled multi-dimensional arrays to be used directly within JAX JIT-compiled functions and grad transforms.

### [dm_pix](https://github.com/google-deepmind/dm_pix)
**★ 438 · `active` · pushed 2025-03 · Apache-2.0**  
PIX is an image processing library in JAX providing augmentation, color transforms, and perceptual metrics that operate on JAX arrays and are JIT/vmap compatible.

### [mishax](https://github.com/google-deepmind/mishax)
**★ 157 · `active` · pushed 2026-02 · Apache-2.0**  
Mechanistic interpretability tools in JAX for extracting and analyzing transformer internals: activation patching, causal tracing, and probing utilities.

### [mctx](https://github.com/google-deepmind/mctx)
**★ 2626 · `active` · pushed 2025-09 · Apache-2.0**  
Topics: `jax` `monte-carlo-tree-search` `reinforcement-learning`  
Monte Carlo tree search in JAX. Provides batched MCTS and AlphaZero-style planning with GPU-accelerated search operations. Used in MuZero and Gumbel MuZero implementations.

### [disentangled_rnns](https://github.com/google-deepmind/disentangled_rnns)
**★ 43 · `active` · pushed 2026-04 · Apache-2.0**  
Disentangled RNN representations in JAX: separates context and memory components in recurrent models for improved generalization across tasks.

### [spectral_ssm](https://github.com/google-deepmind/spectral_ssm)
**★ 35 · `active` · pushed 2024-04 · Apache-2.0**  
Spectral State Space Models in JAX: sequence models using learnable spectral filters as alternatives to attention or recurrence.

## Other repos in this theme
| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [dm_aux](https://github.com/google-deepmind/dm_aux) | 67 | active | Audio processing utilities in JAX |
| [alta](https://github.com/google-deepmind/alta) | 31 | active | Transformers expressed as programs (JAX) |
| [thunnini](https://github.com/google-deepmind/thunnini) | 10 | active | Experimentation library for fine-tuning neural sequential predictors |
