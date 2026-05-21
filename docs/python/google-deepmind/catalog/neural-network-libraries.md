# Google DeepMind · Neural Network Libraries

TensorFlow-era and framework-agnostic neural network libraries, graph learning, sequence models, and research implementations of published architectures.

> Part of [`docs/python/google-deepmind/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 46 repos (40 active / 6 archived).

## TensorFlow neural network libraries

### [sonnet](https://github.com/google-deepmind/sonnet)
**★ 9918 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `artificial-intelligence` `deep-learning` `machine-learning` `neural-networks` `tensorflow`  
TensorFlow-based neural network library providing a module system (`snt.Module`) for building and reusing parameterized network components. The primary DeepMind TF research library, preceding and complementing dm-haiku for JAX.

### [graph_nets](https://github.com/google-deepmind/graph_nets)
**★ 5401 · `active` · pushed 2022-12 · Apache-2.0**  
Build Graph Nets in TensorFlow. Reference implementation from "Relational Inductive Biases, Deep Learning, and Graph Networks" (Battaglia et al., 2018). Provides message passing and global pooling for arbitrary graph structures.

### [kinetics-i3d](https://github.com/google-deepmind/kinetics-i3d)
**★ 1834 · `active` · pushed 2019-09 · Apache-2.0**  
Inflated 3D ConvNet (I3D) model for video classification trained on Kinetics-400 and Kinetics-600. Widely used as a backbone for action recognition, anomaly detection, and video understanding research.

### [dnc](https://github.com/google-deepmind/dnc)
**★ 2540 · `active` · pushed 2021-07 · Apache-2.0**  
TensorFlow implementation of the Differentiable Neural Computer (Graves et al., Nature 2016). Combines a neural network with an external differentiable memory matrix, enabling complex algorithmic task learning.

## Sequence and language models

### [lamb](https://github.com/google-deepmind/lamb)
**★ 138 · `archived` · pushed 2020-04 · Apache-2.0**  
LAnguage Modelling Benchmarks: TensorFlow implementations of LSTM language models used to establish state-of-the-art results on Penn Treebank and other benchmarks (circa 2018-2019).

### [transformer_grammars](https://github.com/google-deepmind/transformer_grammars)
**★ 137 · `active` · pushed 2026-05 · Apache-2.0**  
Transformer Grammars: augmenting transformer language models with syntactic inductive biases at scale. Code for TACL 2022 paper.

### [spectral_inference_networks](https://github.com/google-deepmind/spectral_inference_networks)
**★ 171 · `archived` · pushed 2019-05 · Apache-2.0**  
TensorFlow implementation of Spectral Inference Networks (ICLR 2019) for learning eigenfunctions of positive semi-definite operators.

### [dynamic-kanerva-machines](https://github.com/google-deepmind/dynamic-kanerva-machines)
**★ 44 · `active` · pushed 2019-01 · Apache-2.0**  
Self-contained memory module for the Dynamic Kanerva Machine (NeurIPS 2018), modeling episodic memory with fast write and slow read mechanisms.

### [grid-cells](https://github.com/google-deepmind/grid-cells)
**★ 263 · `active` · pushed 2020-10 · Apache-2.0**  
Supervised learning implementation from "Vector-based Navigation using Grid-like Representations in Artificial Agents" (Nature 2018). Models hippocampal grid cells emerging from navigation tasks.

### [neural_networks_chomsky_hierarchy](https://github.com/google-deepmind/neural_networks_chomsky_hierarchy)
**★ 218 · `active` · pushed 2024-04 · Apache-2.0**  
Research code for "Neural Networks and the Chomsky Hierarchy" (ICLR 2023), evaluating which formal language classes different NN architectures can recognize.

## Graph and structured prediction

### [digraph_transformer](https://github.com/google-deepmind/digraph_transformer)
**★ 122 · `active` · pushed 2023-09 · Other**  
Transformer architecture for directed graphs, processing edges with directional attention masks. Used for program analysis and combinatorial reasoning.

### [gnn_single_rigids](https://github.com/google-deepmind/gnn_single_rigids)
**★ 7 · `active` · pushed 2026-03 · Apache-2.0**  
GNN-based simulation of rigid body dynamics using particle-level representations.

## Verification and robustness

### [interval-bound-propagation](https://github.com/google-deepmind/interval-bound-propagation)
**★ 161 · `active` · pushed 2019-12 · Apache-2.0**  
Simple TensorFlow implementation of Interval Bound Propagation (IBP) for certifying neural network robustness against adversarial perturbations within L-inf norm balls.

### [deep-verify](https://github.com/google-deepmind/deep-verify)
**★ 19 · `active` · pushed 2019-11 · Apache-2.0**  
Verification tools for deep neural networks including LP relaxation and mixed-integer programming-based methods.

## Vision and media models

### [videoprism](https://github.com/google-deepmind/videoprism)
**★ 372 · `active` · pushed 2026-05 · Apache-2.0**  
VideoPrism: foundational visual encoder for video understanding (ICML 2024 oral). Pre-trained on 36M video-caption pairs, achieves strong zero-shot and fine-tuned performance on video classification, QA, and captioning.

### [dmvr](https://github.com/google-deepmind/dmvr)
**★ 69 · `active` · pushed 2022-11 · Apache-2.0**  
DeepMind Video Research library: video data loading and preprocessing pipelines for TensorFlow, supporting common video benchmarks with efficient sharding.

### [brave](https://github.com/google-deepmind/brave)
**★ 51 · `active` · pushed 2026-03 · Apache-2.0**  
JAX implementation of "Broaden Your Views for Self-Supervised Video Learning" (BraVe): uses multiple temporal views of video with bootstrapped contrastive learning.

### [detcon](https://github.com/google-deepmind/detcon)
**★ 62 · `archived` · pushed 2022-10 · Apache-2.0**  
DetCon: self-supervised pre-training that aligns contrastive learning with detection-style object masks.

### [slowfast_nfnets](https://github.com/google-deepmind/slowfast_nfnets)
**★ 30 · `archived` · pushed 2022-06 · Apache-2.0**  
SlowFast NFNet video models combining Normalizer-Free Networks with SlowFast temporal processing for action recognition.

### [magiclens](https://github.com/google-deepmind/magiclens)
**★ 210 · `active` · pushed 2024-10 · Apache-2.0**  
MagicLens: self-supervised image retrieval with open-ended natural language instructions (ICML 2024 Oral). Trains a dual-encoder without human labels by mining compositional image pairs.

### [trecvit](https://github.com/google-deepmind/trecvit)
**★ 26 · `active` · pushed 2026-01 · Apache-2.0**  
TrecViT: video recognition architecture combining Token Reduction with Vision Transformers for efficient long-video understanding.

### [pix2act](https://github.com/google-deepmind/pix2act)
**★ 60 · `active` · pushed 2024-01 · Apache-2.0**  
Pix2Act: pixel-to-action agent for web navigation using screenshot observations and structured action generation with a seq2seq transformer.

### [action_piece](https://github.com/google-deepmind/action_piece)
**★ 58 · `active` · pushed 2026-04 · Apache-2.0**  
Action tokenization for generalist robot policies using learned discrete action representations (action pieces) analogous to text tokenization.

### [proactive_t2i_agents](https://github.com/google-deepmind/proactive_t2i_agents)
**★ 71 · `active` · pushed 2025-07 · Apache-2.0**  
Proactive Agents for Text-to-Image Generation Under Uncertainty: agents that ask clarifying questions before generating images to resolve ambiguous prompts.

### [platonic_rep_video](https://github.com/google-deepmind/platonic_rep_video)
**★ 6 · `active` · pushed 2026-04 · Apache-2.0**  
Research on the Platonic Representation Hypothesis applied to video models: alignment of visual representations across different architectures.

### [serial_depth](https://github.com/google-deepmind/serial_depth)
**★ 18 · `active` · pushed 2026-03 · Apache-2.0**  
Serial Depth: monocular depth estimation using autoregressive image generation as a structured prediction framework.

## Generalization and representation

### [functa](https://github.com/google-deepmind/functa)
**★ 161 · `active` · pushed 2024-07 · Apache-2.0**  
Functa: representing functions as data using implicit neural representations (INRs). Learns per-sample function modulations that encode signals as latent vectors usable for downstream tasks.

### [hierarchical_perceiver](https://github.com/google-deepmind/hierarchical_perceiver)
**★ 32 · `active` · pushed 2026-05 · Apache-2.0**  
Hierarchical Perceiver: multi-scale Perceiver IO architecture for processing structured inputs at different resolutions.

### [nanodo](https://github.com/google-deepmind/nanodo)
**★ 307 · `active` · pushed 2024-07 · Apache-2.0**  
Minimal transformer decoder implementation in JAX for rapid experimentation with decoder-only language model architectures.

### [tracr](https://github.com/google-deepmind/tracr)
**★ 565 · `archived` · pushed 2024-02 · Apache-2.0**  
TRACR: Compiled Transformers as a Laboratory for Interpretability. Compiles programs into transformer weights with known internal representations for mechanistic interpretability research.

### [ssl_hsic](https://github.com/google-deepmind/ssl_hsic)
**★ 39 · `active` · pushed 2024-07 · Apache-2.0**  
Self-supervised learning via Hilbert-Schmidt Independence Criterion: information-theoretic objective for contrastive representation learning.

### [relicv2](https://github.com/google-deepmind/relicv2)
**★ 5 · `active` · pushed 2022-12 · Apache-2.0**  
ReLIC v2: self-supervised invariant representation learning via relaxed contrastive loss.

## Text and language analysis

### [codoc](https://github.com/google-deepmind/codoc)
**★ 120 · `archived` · pushed 2023-07 · Apache-2.0**  
CoDo-C: co-training with document context for medical code prediction from clinical notes.

### [image_obfuscation_benchmark](https://github.com/google-deepmind/image_obfuscation_benchmark)
**★ 27 · `active` · pushed 2026-05 · Apache-2.0**  
Benchmark evaluating model robustness to various image obfuscation types (cropping, masking, blurring) including adversarial perturbations.

### [multi_object_datasets](https://github.com/google-deepmind/multi_object_datasets)
**★ 288 · `active` · pushed 2026-03 · Apache-2.0**  
Multi-object image datasets with ground-truth segmentation masks and generative factors for evaluating object-centric representation learning.

### [gqn-datasets](https://github.com/google-deepmind/gqn-datasets)
**★ 274 · `active` · pushed 2022-02 · Apache-2.0**  
Datasets used to train Generative Query Networks (GQNs) for neural scene representation and novel view synthesis.

### [cube](https://github.com/google-deepmind/cube)
**★ 8 · `active` · pushed 2025-01 · Apache-2.0**  
CUBE extraction and cultural diversity metric from "Beyond Aesthetics: Cultural Competence in Text-to-Image Models", measuring representation of global cultures.

### [c3_neural_compression](https://github.com/google-deepmind/c3_neural_compression)
**★ 99 · `active` · pushed 2026-04 · Apache-2.0**  
C3: high-fidelity neural image compression using JAX. Implements a competitive learned codec combining entropy coding with neural synthesis transforms.

## Other repos in this theme
| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [lm_act](https://github.com/google-deepmind/lm_act) | 28 | active | LMAct benchmark for in-context imitation learning with multimodal demonstrations |
| [calm](https://github.com/google-deepmind/calm) | 58 | active | Composable LLM-augmented motion policies |
| [predictingthepast](https://github.com/google-deepmind/predictingthepast) | 195 | active | Predicting the past via temporal self-supervision |
| [threednel](https://github.com/google-deepmind/threednel) | 18 | active | 3D neural object representations for learning |
| [geomatch](https://github.com/google-deepmind/geomatch) | 18 | active | Geometric matching and 3D correspondence |
| [sam_edge](https://github.com/google-deepmind/sam_edge) | 24 | active | SAM-based edge detection |
| [Temporal-3D-Pose-Kinetics](https://github.com/google-deepmind/Temporal-3D-Pose-Kinetics) | 227 | active | 3D human pose estimation from Kinetics using temporal context |
| [physics-IQ-benchmark](https://github.com/google-deepmind/physics-IQ-benchmark) | 293 | active | Benchmarking physical understanding in generative video models |
