# Google DeepMind · Foundation Models and AlphaX

Foundational model releases and the AlphaX programme: protein structure prediction, geometric reasoning, mathematical discovery, language models, and weather forecasting.

> Part of [`docs/python/google-deepmind/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 24 repos (23 active / 1 archived).

## Protein structure prediction

### [alphafold](https://github.com/google-deepmind/alphafold)
**★ 14605 · `active` · pushed 2026-04 · Apache-2.0**  
Open-source implementation of AlphaFold 2 for protein structure prediction. Companion to the 2021 Nature publication. The most-starred repo in the google-deepmind org. Predicts 3D protein structure from amino-acid sequence using attention-based architectures trained on PDB and genetic databases.

### [alphafold3](https://github.com/google-deepmind/alphafold3)
**★ 8054 · `active` · pushed 2026-05 · Other**  
Inference pipeline for AlphaFold 3, extending structure prediction to RNA, DNA, small molecules, and protein complexes via a diffusion-based architecture. Released November 2024 under a custom non-commercial research license.

### [alphagenome](https://github.com/google-deepmind/alphagenome)
**★ 1924 · `active` · pushed 2026-05 · Apache-2.0**  
Programmatic API for the AlphaGenome model, which predicts genomic tracks (gene expression, chromatin accessibility, splicing) from raw DNA sequence at 128 bp resolution across the full 1 Mb context window.

### [alphagenome_research](https://github.com/google-deepmind/alphagenome_research)
**★ 763 · `active` · pushed 2026-05 · Apache-2.0**  
Research code accompanying AlphaGenome. Contains training scripts, fine-tuning examples, and analysis notebooks used in the original publication.

## Mathematical and algorithmic reasoning

### [alphageometry](https://github.com/google-deepmind/alphageometry)
**★ 4841 · `active` · pushed 2026-01 · Apache-2.0**  
DeepMind's system for automated theorem proving in Euclidean geometry, achieving IMO gold-medallist level on olympiad problems. Combines a language model with a symbolic deduction engine (DDAR).

### [alphageometry2](https://github.com/google-deepmind/alphageometry2)
**★ 73 · `active` · pushed 2026-01 · Apache-2.0**  
AlphaGeometry 2 symbolic engine (DDAR) with usage examples. Extends AlphaGeometry with improved search and a more expressive language.

### [alphatensor](https://github.com/google-deepmind/alphatensor)
**★ 2838 · `active` · pushed 2024-04 · Apache-2.0**  
Code for AlphaTensor, which uses RL to discover novel fast matrix multiplication algorithms (Nature 2022). Demonstrates AI-driven discovery of improved algorithms for fundamental linear algebra operations.

### [alphatensor_quantum](https://github.com/google-deepmind/alphatensor_quantum)
**★ 54 · `active` · pushed 2025-03 · Apache-2.0**  
Extension of AlphaTensor applied to quantum circuit optimization. Discovers T-gate-efficient matrix multiplication algorithms relevant to fault-tolerant quantum computing.

### [alphadev](https://github.com/google-deepmind/alphadev)
**★ 738 · `archived` · pushed 2023-06 · Apache-2.0**  
Code accompanying AlphaDev (Nature 2023), which used RL to discover improved sorting and hashing algorithms directly in assembly language by framing algorithm discovery as a game.

### [alphastar](https://github.com/google-deepmind/alphastar)
**★ 567 · `active` · pushed 2022-09 · Apache-2.0**  
Reference implementation for AlphaStar, the agent that reached Grandmaster level at StarCraft II. Includes architecture definitions and replay analysis tools.

### [searchless_chess](https://github.com/google-deepmind/searchless_chess)
**★ 631 · `active` · pushed 2025-01 · Apache-2.0**  
Demonstrates grandmaster-level chess play without explicit tree search, using a large transformer trained directly on Stockfish evaluations to predict actions and values.

## Language models (Gemma family)

### [gemma](https://github.com/google-deepmind/gemma)
**★ 5239 · `active` · pushed 2026-05 · Apache-2.0**  
JAX/Flax reference implementation of the Gemma open-weight language models (2B, 7B, 9B, 27B parameters). Covers inference, fine-tuning, and RLHF recipes. The canonical research codebase for the Gemma model family.

### [recurrentgemma](https://github.com/google-deepmind/recurrentgemma)
**★ 677 · `active` · pushed 2026-02 · Apache-2.0**  
Open-weights language model based on the Griffin architecture (linear recurrence + local attention), offering efficient inference with lower memory cost than pure attention models at the same parameter count.

### [simply](https://github.com/google-deepmind/simply)
**★ 526 · `active` · pushed 2026-05 · Apache-2.0**  
Minimal and scalable JAX research codebase for rapid iteration on frontier LLM and autoregressive model research. Provides clean baseline implementations of transformer training loops.

### [synthid-text](https://github.com/google-deepmind/synthid-text)
**★ 867 · `active` · pushed 2025-08 · Apache-2.0**  
SynthID-Text implements tournament sampling and Bayesian detector for watermarking LLM-generated text without degrading output quality. Companion to the Nature 2024 paper.

## Weather and Earth system modelling

### [graphcast](https://github.com/google-deepmind/graphcast)
**★ 6661 · `active` · pushed 2026-03 · Apache-2.0**  
Topics: `weather` `weather-forecast`  
JAX/Haiku implementation of GraphCast, a graph neural network weather forecasting model achieving state-of-the-art 10-day medium-range forecasts at 0.25-degree resolution. Trained on ERA5 reanalysis data.

## Optimisation-based discovery

### [opro](https://github.com/google-deepmind/opro)
**★ 745 · `active` · pushed 2024-12 · Apache-2.0**  
Official code for "Large Language Models as Optimizers" (ICLR 2024). Uses LLMs to iteratively generate and refine prompt instructions, treating the LLM as a black-box optimizer for few-shot tasks.

### [disco_rl](https://github.com/google-deepmind/disco_rl)
**★ 701 · `active` · pushed 2025-12 · Apache-2.0**  
Topics: `ai` `meta-learning` `nature` `reinforcement-learning` `reinforcement-learning-algorithms`  
Accompanying code for "Discovering State-of-the-Art Reinforcement Algorithms" (Nature). Automated RL algorithm discovery through program synthesis and meta-evolution.

### [regress-lm](https://github.com/google-deepmind/regress-lm)
**★ 338 · `active` · pushed 2026-05 · Apache-2.0**  
Text-to-text regression library supporting any string input representation. Enables pretraining and fine-tuning over multiple regression tasks simultaneously.

### [science-skills](https://github.com/google-deepmind/science-skills)
**★ 185 · `active` · pushed 2026-05 · Apache-2.0**  
GDM Science Skills for agentic scientific workflows integrating AlphaGenome, AFDB, UniProt, and 30+ databases and tools. Optimizes token efficiency for LLM-driven scientific computation.

## Other repos in this theme
| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [concordia](https://github.com/google-deepmind/concordia) | 1433 | active | Library for generative social simulation |
| [habermas_machine](https://github.com/google-deepmind/habermas_machine) | 95 | active | Habermas Machine: AI-facilitated democratic deliberation |
| [onetwo](https://github.com/google-deepmind/onetwo) | 268 | active | LLM composition and prompt programming framework |
| [eval_hub](https://github.com/google-deepmind/eval_hub) | 16 | active | Gemini evaluations hub |
