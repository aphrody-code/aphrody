# Google DeepMind · Science, Datasets, and Tools

Computational science applications (physics, chemistry, biology, climate), published datasets and benchmarks for language and reasoning, and developer tooling.

> Part of [`docs/google-deepmind/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 76 repos (65 active / 11 archived).

## Computational biology and chemistry

### [alphamissense](https://github.com/google-deepmind/alphamissense)
**★ 633 · `archived` · pushed 2024-03 · Apache-2.0**  
Code accompanying AlphaMissense (Science 2023), which classifies all ~71 million possible human missense variants as likely pathogenic or benign using protein language model embeddings combined with structural context from AlphaFold.

### [ferminet](https://github.com/google-deepmind/ferminet)
**★ 837 · `active` · pushed 2026-05 · Apache-2.0**  
Fermionic Neural Network (FermiNet) for ab-initio electronic structure calculations. Solves the many-electron Schrodinger equation using a neural network ansatz that satisfies the antisymmetry constraint. Achieves near-exact results on small molecules.

### [inverse_design](https://github.com/google-deepmind/inverse_design)
**★ 28 · `active` · pushed 2026-03 · Apache-2.0**  
Graph neural network-based molecular inverse design: generates molecule structures satisfying desired property constraints via differentiable optimization.

### [surface-distance](https://github.com/google-deepmind/surface-distance)
**★ 597 · `active` · pushed 2025-02 · Apache-2.0**  
Library for computing surface distance-based segmentation metrics: Average Symmetric Surface Distance, Hausdorff distance, and volumetric Dice for 3D medical image segmentation evaluation.

### [alignet](https://github.com/google-deepmind/alignet)
**★ 76 · `active` · pushed 2025-12 · Apache-2.0**  
AlignNet: neural sequence alignment for biological sequences, implementing differentiable alignment objectives for protein and nucleotide comparison.

## Language, reasoning, and NLP datasets

### [mathematics_dataset](https://github.com/google-deepmind/mathematics_dataset)
**★ 1955 · `active` · pushed 2024-12 · Apache-2.0**  
Generates school-level mathematics question-answer pairs across 57 modules (algebra, arithmetic, calculus, probability, etc.). Widely used to evaluate LLM mathematical reasoning.

### [rc-data](https://github.com/google-deepmind/rc-data)
**★ 1296 · `archived` · pushed 2017-04 · Apache-2.0**  
CNN/DailyMail reading comprehension dataset from "Teaching Machines to Read and Comprehend" (Hermann et al., NeurIPS 2015). One of the first large-scale cloze-style QA datasets.

### [loft](https://github.com/google-deepmind/loft)
**★ 233 · `active` · pushed 2026-04 · Apache-2.0**  
LOFT: 1 Million+ Token Long-Context Benchmark evaluating LLM retrieval, reasoning, and in-context learning over very long documents.

### [long-form-factuality](https://github.com/google-deepmind/long-form-factuality)
**★ 685 · `active` · pushed 2026-05 · Other**  
Benchmark and code for evaluating long-form factuality in LLMs. Introduces SAFE (Search-Augmented Factuality Evaluator) for automated fine-grained factual claim verification.

### [streamingqa](https://github.com/google-deepmind/streamingqa)
**★ 50 · `active` · pushed 2023-10 · Apache-2.0**  
StreamingQA: benchmark for evaluating LLM knowledge update over time with a continuously updated news QA dataset.

### [slim-dataset](https://github.com/google-deepmind/slim-dataset)
**★ 36 · `archived` · pushed 2018-07 · Apache-2.0**  
Spatial Language Integrating Model datasets for training models to encode spatial relations from natural language descriptions.

### [language_modeling_is_compression](https://github.com/google-deepmind/language_modeling_is_compression)
**★ 182 · `active` · pushed 2024-08 · Apache-2.0**  
Code for "Language Modeling Is Compression" (ICLR 2024). Demonstrates LLMs as general-purpose lossless compressors and analyzes compression-prediction duality.

### [randomized_positional_encodings](https://github.com/google-deepmind/randomized_positional_encodings)
**★ 82 · `active` · pushed 2024-03 · Apache-2.0**  
Randomized Positional Encodings Boost Length Generalization of Transformers: training technique that samples position indices randomly to improve extrapolation to unseen sequence lengths.

### [emergent_in_context_learning](https://github.com/google-deepmind/emergent_in_context_learning)
**★ 88 · `active` · pushed 2024-07 · Apache-2.0**  
Emergent In-Context Learning in Transformers: investigates conditions under which in-context learning abilities emerge during pre-training.

### [neural_networks_solomonoff_induction](https://github.com/google-deepmind/neural_networks_solomonoff_induction)
**★ 84 · `archived` · pushed 2024-08 · Apache-2.0**  
Learning Universal Predictors: research on training transformers to approximate Solomonoff induction and universal prediction.

### [llms_can_learn_rules](https://github.com/google-deepmind/llms_can_learn_rules)
**★ 63 · `active` · pushed 2024-12 · Apache-2.0**  
Research demonstrating that LLMs can induce and apply symbolic rules from examples during in-context learning.

### [latent-multi-hop-reasoning](https://github.com/google-deepmind/latent-multi-hop-reasoning)
**★ 91 · `active` · pushed 2025-03 · Apache-2.0**  
ACL 2024 paper: "Do Large Language Models Latently Perform Multi-Hop Reasoning?" Probes transformer hidden states for intermediate reasoning steps in multi-hop QA.

### [bbeh](https://github.com/google-deepmind/bbeh)
**★ 120 · `active` · pushed 2025-05 · Apache-2.0**  
BeyondBIG-Bench Hard (BBEH): harder version of BIG-Bench Hard with more challenging reasoning tasks including temporal reasoning, spatial navigation, and logical deduction.

### [questbench](https://github.com/google-deepmind/questbench)
**★ 38 · `active` · pushed 2025-05 · Apache-2.0**  
QuestBench: benchmark for evaluating whether LLMs ask the right clarifying questions before attempting ambiguous tasks.

### [natural-plan](https://github.com/google-deepmind/natural-plan)
**★ 57 · `active` · pushed 2024-09 · Apache-2.0**  
NATURAL PLAN: multi-step planning benchmark testing LLM ability to solve scheduling, trip planning, and calendar management tasks requiring complex constraint reasoning.

### [tanq](https://github.com/google-deepmind/tanq)
**★ 11 · `active` · pushed 2024-06 · Apache-2.0**  
TANQ: table-augmented natural language question answering benchmark.

### [anthro-benchmark](https://github.com/google-deepmind/anthro-benchmark)
**★ 11 · `active` · pushed 2025-11 · Apache-2.0**  
Anthropomorphism benchmark for evaluating human-like attribution and personification tendencies in LLM outputs.

### [recoglab](https://github.com/google-deepmind/recoglab)
**★ 13 · `active` · pushed 2026-05 · Apache-2.0**  
Recognition Laboratory: visual concept recognition benchmark combining fine-grained classification and few-shot learning tasks.

### [scivid](https://github.com/google-deepmind/scivid)
**★ 16 · `active` · pushed 2026-03 · Apache-2.0**  
Scientific video understanding benchmark for evaluating multimodal models on physics and natural science videos.

## Medical and clinical AI

### [distribution_shift_framework](https://github.com/google-deepmind/distribution_shift_framework)
**★ 87 · `active` · pushed 2026-05 · Apache-2.0**  
Fine-grained analysis of distribution shift: framework for evaluating and comparing model robustness to covariate, label, and concept shift in medical imaging and other domains.

### [nao_top10](https://github.com/google-deepmind/nao_top10)
**★ 19 · `active` · pushed 2023-03 · Apache-2.0**  
Neural Algorithm of Artistic Style (NAO) top-10 results and analysis code.

## Privacy and fairness

### [wasserstein_fairness](https://github.com/google-deepmind/wasserstein_fairness)
**★ 23 · `active` · pushed 2020-01 · Apache-2.0**  
Implementation of Wasserstein Fair Classification (UAI 2019), optimizing demographic parity constraints using optimal transport distances.

### [privately-counting-distinct-elements](https://github.com/google-deepmind/privately-counting-distinct-elements)
**★ 3 · `active` · pushed 2026-04 · Apache-2.0**  
Code for "Counting Distinct Elements Under Person-Level Differential Privacy" (NeurIPS 2024).

### [dangerous-capability-evaluations](https://github.com/google-deepmind/dangerous-capability-evaluations)
**★ 73 · `active` · pushed 2026-05 · Apache-2.0**  
Evaluations for dangerous capabilities in LLMs including CBRN knowledge, cyber-offense, persuasion, and autonomous replication. Used in Gemini safety evaluations.

### [unlearning_evaluation](https://github.com/google-deepmind/unlearning_evaluation)
**★ 18 · `active` · pushed 2025-05 · Apache-2.0**  
Framework for evaluating machine unlearning methods: tests whether models correctly forget targeted training examples while retaining general capabilities.

## Causal inference and Bayesian optimization

### [ccbo](https://github.com/google-deepmind/ccbo)
**★ 15 · `active` · pushed 2024-06 · Apache-2.0**  
Constrained Causal Bayesian Optimization (ICML 2023): BO with causal graph constraints for interventional experimental design.

### [max_product_noisy_or](https://github.com/google-deepmind/max_product_noisy_or)
**★ 10 · `active` · pushed 2026-05 · Apache-2.0**  
Max-product belief propagation for Noisy-OR Bayesian networks, implemented for causal graph inference.

### [proeval](https://github.com/google-deepmind/proeval)
**★ 31 · `active` · pushed 2026-05 · Apache-2.0**  
GenAI evaluation framework optimized for 100x lower cost via efficient sampling and cached evaluations.

## Developer and ML tooling

### [fancyflags](https://github.com/google-deepmind/fancyflags)
**★ 35 · `active` · pushed 2025-07 · Apache-2.0**  
Python library for defining structured command-line flags with nested dataclass support, extending `absl-py` flags.

### [tensor_annotations](https://github.com/google-deepmind/tensor_annotations)
**★ 159 · `archived` · pushed 2023-07 · Apache-2.0**  
Annotating tensor shapes using Python type hints, enabling static shape checking for JAX, NumPy, and TensorFlow arrays.

### [leaps-and-bounds](https://github.com/google-deepmind/leaps-and-bounds)
**★ 8 · `active` · pushed 2020-01 · Apache-2.0**  
Implementation of LeapsAndBounds and Structured Procrastination for approximately optimal algorithm configuration.

### [batch-isolation-checker](https://github.com/google-deepmind/batch-isolation-checker)
**★ 5 · `active` · pushed 2026-03 · Apache-2.0**  
Tool for checking batch isolation in distributed ML training pipelines, detecting cross-batch data contamination.

## Neural architecture search and optimization

### [neural_lns](https://github.com/google-deepmind/neural_lns)
**★ 51 · `active` · pushed 2022-02 · Apache-2.0**  
Neural Large Neighbourhood Search: learned heuristics for combinatorial optimization via RL-guided neighborhood exploration in mixed-integer programming.

### [eigengame](https://github.com/google-deepmind/eigengame)
**★ 35 · `archived` · pushed 2023-05 · Apache-2.0**  
EigenGame: computes principal components as a Nash equilibrium of a game between player-vectors, enabling distributed PCA at scale.

### [autonumerics_zero](https://github.com/google-deepmind/autonumerics_zero)
**★ 11 · `active` · pushed 2026-04 · Apache-2.0**  
AutoNumerics-Zero: automated discovery of numerical algorithms via program synthesis, targeting fast approximations of transcendental functions.

### [iris](https://github.com/google-deepmind/iris)
**★ 21 · `active` · pushed 2025-06 · Apache-2.0**  
IRIS: interleaved reinforcement learning and imitation for sequential decision making with sparse rewards.

### [egg](https://github.com/google-deepmind/egg)
**★ 20 · `active` · pushed 2026-04 · Apache-2.0**  
EGG (Emergent language/Game framework): toolkit for research on emergent communication in multi-agent language games.

## Research paper code (misc)

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [dm_c19_modelling](https://github.com/google-deepmind/dm_c19_modelling) | 7 | archived | COVID-19 epidemiological modelling code |
| [emergent_communication_at_scale](https://github.com/google-deepmind/emergent_communication_at_scale) | 39 | active | Large-scale emergent communication experiments |
| [nonstationary_mbml](https://github.com/google-deepmind/nonstationary_mbml) | 17 | active | Memory-Based Meta-Learning on Non-Stationary Distributions |
| [active_ops](https://github.com/google-deepmind/active_ops) | 33 | active | Active learning operators for efficient data collection |
| [nfg_transformer](https://github.com/google-deepmind/nfg_transformer) | 9 | active | Transformer for normal-form games (game theory) |
| [additive_cbug](https://github.com/google-deepmind/additive_cbug) | 5 | archived | Additive combinatorial bandits under general uncertainty |
| [abcei_mab](https://github.com/google-deepmind/abcei_mab) | 5 | active | Approximately Bayesian contextual-epsilon-insensitive multi-armed bandits |
| [protex](https://github.com/google-deepmind/protex) | 20 | active | Prototype-based explanation for neural network predictions |
| [local_linearity_regularizer](https://github.com/google-deepmind/local_linearity_regularizer) | 4 | active | Local linearity regularization for improved adversarial robustness |
| [codesembench](https://github.com/google-deepmind/codesembench) | 16 | archived | Code semantic similarity benchmark |
| [icml2024-roundtrip-correctness](https://github.com/google-deepmind/icml2024-roundtrip-correctness) | 16 | archived | Round-trip correctness evaluation for code generation (ICML 2024) |
| [exedec](https://github.com/google-deepmind/exedec) | 14 | active | ExeDec: decomposition-based code synthesis using execution traces |
| [mammut](https://github.com/google-deepmind/mammut) | 4 | active | Massively Multimodal Universal Training framework |
| [amld_workshop_natural_interactions_with_llms](https://github.com/google-deepmind/amld_workshop_natural_interactions_with_llms) | 10 | active | Workshop materials for natural interactions with LLMs |
| [what_type_of_inference_is_planning](https://github.com/google-deepmind/what_type_of_inference_is_planning) | 3 | active | NeurIPS 2024: planning as probabilistic inference |
| [simulation_streams](https://github.com/google-deepmind/simulation_streams) | 25 | active | Programming paradigm for LLM-driven agentic simulations |
| [neural_assets](https://github.com/google-deepmind/neural_assets) | 22 | active | Neural asset compression and synthesis |
| [corr_faith](https://github.com/google-deepmind/corr_faith) | 6 | active | Correlation-faithfulness trade-off in explanations |
| [atomic_concept_edits](https://github.com/google-deepmind/atomic_concept_edits) | 6 | active | Atomic concept editing in neural networks |
| [mona](https://github.com/google-deepmind/mona) | 6 | active | Model-based online network adaptation |
| [mir_uai25](https://github.com/google-deepmind/mir_uai25) | 5 | active | Mutual information regularization (UAI 2025) |
| [wtos_agglabels_uai25](https://github.com/google-deepmind/wtos_agglabels_uai25) | 2 | active | Weak-to-strong label aggregation (UAI 2025) |
| [agg_data_uai25](https://github.com/google-deepmind/agg_data_uai25) | 2 | active | Aggregated data experiments (UAI 2025) |
| [llp_bp](https://github.com/google-deepmind/llp_bp) | 2 | active | Learning from Label Proportions via belief propagation |
| [covariate_shifted_llp](https://github.com/google-deepmind/covariate_shifted_llp) | 2 | active | Learning from Label Proportions under covariate shift (UAI 2025) |
| [fractal_acl25](https://github.com/google-deepmind/fractal_acl25) | 2 | active | Fractal compositional generalization (ACL 2025) |
| [ddgc](https://github.com/google-deepmind/ddgc) | 2 | active | Denoising diffusion for graph completion |
| [beneath-the-surface](https://github.com/google-deepmind/beneath-the-surface) | 3 | active | Subword tokenization analysis |
| [unpuzzles_and_simple_reasoning](https://github.com/google-deepmind/unpuzzles_and_simple_reasoning) | 3 | active | Un-Puzzles: simple reasoning evaluation |
| [utm](https://github.com/google-deepmind/utm) | 3 | active | Universal Turing Machine simulation in neural networks |
| [rem](https://github.com/google-deepmind/rem) | 8 | active | Reward evaluation metrics (GPL-3.0) |
| [polarix](https://github.com/google-deepmind/polarix) | 5 | active | Polarity-aware representation learning |
| [meqpy](https://github.com/google-deepmind/meqpy) | 7 | active | Multi-equation query Python library |
| [semppl](https://github.com/google-deepmind/semppl) | 10 | archived | Semantic probabilistic programming language |
