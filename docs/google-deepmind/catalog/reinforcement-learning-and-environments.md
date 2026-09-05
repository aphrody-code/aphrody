# Google DeepMind · Reinforcement Learning and Environments

Research libraries, agent implementations, multi-agent environments, and evaluation suites for reinforcement learning.

> Part of [`docs/google-deepmind/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 52 repos (45 active / 7 archived).

## Agent libraries and frameworks

### [acme](https://github.com/google-deepmind/acme)
**★ 3986 · `active` · pushed 2026-04 · Apache-2.0**  
Topics: `agents` `reinforcement-learning` `research`  
Production-grade library of RL components and agents (D4PG, IMPALA, R2D2, MuZero, etc.) designed for distributed multi-actor architectures. Supports JAX and TensorFlow backends. The primary DeepMind RL research framework.

### [rlax](https://github.com/google-deepmind/rlax)
**★ 1426 · `active` · pushed 2026-03 · Apache-2.0**  
JAX library of RL building blocks: return estimation, policy gradient estimators, Q-learning updates, distributional RL, intrinsic motivation, and exploration utilities.

### [trfl](https://github.com/google-deepmind/trfl)
**★ 3134 · `active` · pushed 2022-12 · Apache-2.0**  
TensorFlow Reinforcement Learning: earlier library of RL loss functions and utilities (Q-learning, policy gradient, actor-critic). Superseded by acme/rlax but widely referenced.

### [dqn_zoo](https://github.com/google-deepmind/dqn_zoo)
**★ 501 · `active` · pushed 2026-05 · Apache-2.0**  
Reference implementations of DQN and variants (C51, Rainbow, QR-DQN, IQN, Munchausen) from DeepMind's Rainbow paper and sequels. Minimal, reproducible codebase.

### [scalable_agent](https://github.com/google-deepmind/scalable_agent)
**★ 1025 · `active` · pushed 2019-03 · Apache-2.0**  
TensorFlow implementation of IMPALA: Scalable Distributed Deep-RL with Importance Weighted Actor-Learner Architectures. Reference for the V-trace off-policy correction algorithm.

### [enn_acme](https://github.com/google-deepmind/enn_acme)
**★ 30 · `active` · pushed 2022-08 · Apache-2.0**  
Integration of Epistemic Neural Networks (ENNs) with the Acme framework for uncertainty-aware RL agents.

## Physics-based simulation (MuJoCo)

### [dm_control](https://github.com/google-deepmind/dm_control)
**★ 4586 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `artificial-intelligence` `deep-learning` `machine-learning` `mujoco` `neural-networks` `physics-simulation` `reinforcement-learning`  
DeepMind's software stack for physics-based simulation using MuJoCo. Includes the Control Suite benchmark tasks, a composable task construction API, and viewers for headless and interactive rendering.

### [mujoco_menagerie](https://github.com/google-deepmind/mujoco_menagerie)
**★ 3485 · `active` · pushed 2026-05 · Other**  
Topics: `mujoco` `robotics`  
Curated collection of high-quality robot and object models for MuJoCo: industrial arms, humanoids, quadrupeds, hands, and grippers. Each model is carefully validated for physical accuracy.

### [mujoco_warp](https://github.com/google-deepmind/mujoco_warp)
**★ 1240 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `mujoco-warp` `nvidia-warp`  
GPU-optimized MuJoCo physics simulator built on NVIDIA Warp. Enables massively parallel simulation of thousands of MuJoCo environments simultaneously on a single GPU.

### [mujoco_playground](https://github.com/google-deepmind/mujoco_playground)
**★ 1954 · `active` · pushed 2026-05 · Apache-2.0**  
Open-source library for GPU-accelerated robot learning and sim-to-real transfer. Provides JAX-native environments built on MuJoCo for fast vectorized RL training.

### [aloha_sim](https://github.com/google-deepmind/aloha_sim)
**★ 313 · `active` · pushed 2025-11 · Other**  
Collection of tabletop manipulation tasks in MuJoCo designed around the ALOHA robotic platform (bi-manual 6-DOF arms).

### [rgb_stacking](https://github.com/google-deepmind/rgb_stacking)
**★ 129 · `active` · pushed 2024-07 · Apache-2.0**  
Robotic block stacking benchmark in MuJoCo with procedurally generated object geometries and colors, designed for evaluating manipulation generalization.

### [language_to_reward_2023](https://github.com/google-deepmind/language_to_reward_2023)
**★ 159 · `active` · pushed 2024-08 · Apache-2.0**  
Language-to-Reward (L2R): uses LLMs to generate MuJoCo reward functions from natural language task descriptions, enabling zero-shot robot task specification.

### [gemini-robotics-sdk](https://github.com/google-deepmind/gemini-robotics-sdk)
**★ 582 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `gemini` `robotics`  
SDK for integrating Gemini models into robotic control pipelines. Provides interfaces for vision-language-action models and sensor/actuator bridges.

### [dm_robotics](https://github.com/google-deepmind/dm_robotics)
**★ 411 · `archived` · pushed 2026-03 · Apache-2.0**  
Libraries, tools, and tasks created at DeepMind Robotics: manipulation environments, waypoint-based control primitives, and RGB-image-based task definitions built on dm_control.

## Multi-agent environments

### [meltingpot](https://github.com/google-deepmind/meltingpot)
**★ 836 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `multiagent-reinforcement-learning`  
Suite of test scenarios for multi-agent RL covering cooperation, competition, and coordination problems. Provides 50+ social dilemma and coordination substrates with substrate-level evaluation.

### [pysc2](https://github.com/google-deepmind/pysc2)
**★ 8292 · `active` · pushed 2024-07 · Apache-2.0**  
Topics: `blizzard-api` `deepmind` `machine-learning` `reinforcement-learning` `starcraft-ii` `starcraft-ii-replays`  
StarCraft II Learning Environment. Python interface to the SC2 game with observation feature layers, action spaces, and replay processing tools.

### [android_env](https://github.com/google-deepmind/android_env)
**★ 1219 · `active` · pushed 2026-05 · Apache-2.0**  
Topics: `android` `reinforcement-learning`  
RL research environment for Android devices. Exposes Android UI as an RL environment with pixel observations and touch/swipe action spaces, enabling general-purpose GUI agents.

### [hanabi-learning-environment](https://github.com/google-deepmind/hanabi-learning-environment)
**★ 668 · `archived` · pushed 2023-02 · Apache-2.0**  
Research platform for the cooperative card game Hanabi, used extensively in multi-agent theory-of-mind research.

### [diplomacy](https://github.com/google-deepmind/diplomacy)
**★ 60 · `active` · pushed 2024-04 · Apache-2.0**  
Multi-agent negotiation environment for the Diplomacy board game, used in research on communication-based cooperation and competitive agents.

### [game_arena](https://github.com/google-deepmind/game_arena)
**★ 106 · `active` · pushed 2026-02 · Apache-2.0**  
Kaggle-linked evaluation framework for comparing LLM and RL agents across structured games. Companion to the Kaggle Game Arena competition.

## Evaluation and benchmarks

### [bsuite](https://github.com/google-deepmind/bsuite)
**★ 1544 · `active` · pushed 2026-03 · Apache-2.0**  
Behavior Suite for RL: 23 carefully-designed experiments probing core agent capabilities (memory, exploration, credit assignment, noise robustness). Produces radar charts for systematic capability comparison.

### [dm_memorytasks](https://github.com/google-deepmind/dm_memorytasks)
**★ 226 · `active` · pushed 2021-08 · Apache-2.0**  
13 diverse memory-requiring ML tasks implemented in DeepMind Lab 2D, testing episodic, working, and associative memory.

### [dm_hard_eight](https://github.com/google-deepmind/dm_hard_eight)
**★ 85 · `active` · pushed 2020-11 · Apache-2.0**  
Eight challenging 3D navigation and manipulation tasks from DeepMind Lab requiring long-horizon planning and memory.

### [dm_nevis](https://github.com/google-deepmind/dm_nevis)
**★ 102 · `active` · pushed 2022-12 · Apache-2.0**  
NEVIS'22 benchmark for never-ending visual learning, testing agents' ability to continually acquire new visual recognition capabilities without catastrophic forgetting.

### [csuite](https://github.com/google-deepmind/csuite)
**★ 47 · `active` · pushed 2024-09 · Apache-2.0**  
Continuing Tasks Suite: RL benchmark tasks formulated as infinite-horizon MDPs to evaluate average-reward RL algorithms.

### [pushworld](https://github.com/google-deepmind/pushworld)
**★ 94 · `active` · pushed 2026-05 · Apache-2.0**  
Benchmark for manipulation planning with tools and movable obstacles. Evaluates planning under physical constraints with combinatorial state spaces.

### [dmc_vision_benchmark](https://github.com/google-deepmind/dmc_vision_benchmark)
**★ 32 · `active` · pushed 2024-06 · Apache-2.0**  
Vision-based benchmark built on DeepMind Control Suite tasks, evaluating pixel-based RL policies on systematic visual distribution shifts.

## Gridworld and toy environments

### [pycolab](https://github.com/google-deepmind/pycolab)
**★ 664 · `active` · pushed 2019-09 · Apache-2.0**  
Highly customizable gridworld game engine for building custom RL environments. ASCII-based world specification with batteries included.

### [ai-safety-gridworlds](https://github.com/google-deepmind/ai-safety-gridworlds)
**★ 633 · `archived` · pushed 2022-05 · Apache-2.0**  
Suite of RL environments illustrating AI safety properties: interruptibility, safe interruptibility, reward tampering, and side-effects avoidance.

### [spriteworld](https://github.com/google-deepmind/spriteworld)
**★ 373 · `active` · pushed 2020-06 · Apache-2.0**  
Flexible configurable Python-based RL environment for object-centric representation learning research, with composable scene generation.

### [dm_alchemy](https://github.com/google-deepmind/dm_alchemy)
**★ 204 · `archived` · pushed 2023-04 · Apache-2.0**  
DeepMind Alchemy: meta-RL benchmark where agents must infer the latent transformation rules of a potion-crafting task across episodes.

### [dm_construction](https://github.com/google-deepmind/dm_construction)
**★ 28 · `archived` · pushed 2021-01 · Apache-2.0**  
Block construction task environment testing physical intuition and planning in a continuous physics simulation.

### [zipfian_environments](https://github.com/google-deepmind/zipfian_environments)
**★ 28 · `active` · pushed 2022-07 · Apache-2.0**  
RL environments with Zipfian (power-law) reward distributions, studying agent behavior under natural task frequency imbalance.

### [dm_fast_mapping](https://github.com/google-deepmind/dm_fast_mapping)
**★ 54 · `active` · pushed 2021-10 · Apache-2.0**  
DeepMind Fast Mapping environments for evaluating one-shot concept acquisition by RL agents.

### [dm_hamiltonian_dynamics_suite](https://github.com/google-deepmind/dm_hamiltonian_dynamics_suite)
**★ 36 · `active` · pushed 2021-11 · Apache-2.0**  
Suite of physics simulation environments with known Hamiltonian dynamics for evaluating energy-conserving representation learning.

## Environment utilities

### [dm_env](https://github.com/google-deepmind/dm_env)
**★ 400 · `active` · pushed 2022-12 · Apache-2.0**  
Minimal Python interface specification for RL environments. Defines `TimeStep`, `StepType`, and `Environment` ABC used across DeepMind's environment ecosystem.

### [dm_env_rpc](https://github.com/google-deepmind/dm_env_rpc)
**★ 109 · `active` · pushed 2026-02 · Apache-2.0**  
gRPC-based networking protocol for agent-environment communication. Allows environments to run in separate processes or on remote machines while exposing the dm_env interface.

### [envlogger](https://github.com/google-deepmind/envlogger)
**★ 116 · `active` · pushed 2026-04 · Apache-2.0**  
Tool for recording RL trajectories. Wraps dm_env environments to write episodes to efficient RLDS-format storage for offline RL and imitation learning.

### [launchpad](https://github.com/google-deepmind/launchpad)
**★ 330 · `archived` · pushed 2023-08 · Apache-2.0**  
Distributed program construction framework for multi-actor RL systems. Provides abstractions for nodes and edges in distributed training graphs.

## Meta-learning

### [learning-to-learn](https://github.com/google-deepmind/learning-to-learn)
**★ 4070 · `active` · pushed 2021-06 · Apache-2.0**  
Original TensorFlow implementation of "Learning to Learn by Gradient Descent by Gradient Descent" (Andrychowicz et al., 2016). Trains a LSTM to act as an optimizer for other networks.

### [leo](https://github.com/google-deepmind/leo)
**★ 311 · `active` · pushed 2019-04 · Apache-2.0**  
Implementation of Meta-Learning with Latent Embedding Optimization (LEO), a few-shot learning method that learns a low-dimensional latent space for fast adaptation.

### [affordances_option_models](https://github.com/google-deepmind/affordances_option_models)
**★ 22 · `active` · pushed 2021-11 · Apache-2.0**  
Research code for affordance-based option discovery and model-based RL using object affordances as subgoal priors.

### [offpolicy_selection_eslb](https://github.com/google-deepmind/offpolicy_selection_eslb)
**★ 8 · `active` · pushed 2022-03 · Apache-2.0**  
Off-policy policy selection using Empirical Saddlepoint Lower Bounds, providing statistical guarantees for policy comparison from offline data.

### [constrained_optidice](https://github.com/google-deepmind/constrained_optidice)
**★ 10 · `active` · pushed 2022-09 · Apache-2.0**  
Constrained OptiDICE: offline constrained RL with stationary distribution correction estimation.

### [tell_me_why_explanations_rl](https://github.com/google-deepmind/tell_me_why_explanations_rl)
**★ 37 · `archived` · pushed 2023-04 · Apache-2.0**  
Research on generating natural language explanations for RL agent behavior.

### [agent_debugger](https://github.com/google-deepmind/agent_debugger)
**★ 20 · `active` · pushed 2023-06 · Apache-2.0**  
Causal Analysis of Agent Behavior for AI Safety: tools for understanding which environment features causally influence agent decisions.

## Other repos in this theme
| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [sketch_dqn](https://github.com/google-deepmind/sketch_dqn) | 3 | active | Sketch-based DQN distributional RL |
| [tsuite](https://github.com/google-deepmind/tsuite) | 5 | active | Test Suite for RL training pipelines |
| [strategicwm](https://github.com/google-deepmind/strategicwm) | 11 | active | Strategic world models for planning |
| [qtqp](https://github.com/google-deepmind/qtqp) | 31 | active | Quasi-Thompson sampling for efficient exploration |
| [romo](https://github.com/google-deepmind/romo) | 37 | active | Reward model evaluation suite |
