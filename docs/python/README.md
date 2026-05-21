# Google Python open-source — repository catalog

A reference catalog of the Python open-source landscape published by Google across its GitHub organizations, assembled for the aphrody project. Each repo is listed with stars, maintenance status (active vs archived), last-push date and description; notable repos are enriched with real GitHub topics and README excerpts.

## Scope

| Organization | Python repos | Active | Archived | Section |
|--------------|-------------:|-------:|---------:|---------|
| [google](https://github.com/google) | 590 | 194 | 396 | [google/](google/README.md) |
| [google-deepmind](https://github.com/google-deepmind) | 236 | 205 | 31 | [google-deepmind/](google-deepmind/README.md) |
| **Total** | **826** | **399** | **427** | |

## Methodology

- Repository lists pulled via the GitHub Search API (`gh search repos --owner <org> --language python`), snapshot **2026-05-21**. The Search API caps results at 1000; both orgs are under that ceiling, so coverage is complete.
- Each org has an exhaustive, mechanically generated `all-repos.md` table (source of truth: `data/*.json`) plus thematic `catalog/` pages.
- Thematic pages enrich notable repos with GitHub topics and README excerpts fetched live via `gh api`; descriptions are quoted verbatim from GitHub.
- `language:python` reflects each repo's GitHub-detected primary language; polyglot repos whose primary language is not Python are out of scope.

## Top 20 by stars (both orgs)

| # | Repo | Stars | Status | Description |
|--:|------|------:|--------|-------------|
| 1 | [langextract](https://github.com/google/langextract) | 36514 | active | A Python library for extracting structured information from unstructured text using LLMs with … |
| 2 | [python-fire](https://github.com/google/python-fire) | 28193 | active | Python Fire is a library for automatically generating command line interfaces (CLIs) from abso… |
| 3 | [adk-python](https://github.com/google/adk-python) | 19777 | active | An open-source, code-first Python toolkit for building, evaluating, and deploying sophisticate… |
| 4 | [magika](https://github.com/google/magika) | 17033 | active | Fast and accurate AI powered file content types detection |
| 5 | [alphafold](https://github.com/google-deepmind/alphafold) | 14605 | active | Open source code for AlphaFold 2. |
| 6 | [yapf](https://github.com/google/yapf) | 13984 | active | A formatter for Python files |
| 7 | [skills](https://github.com/google/skills) | 10176 | active | Agent Skills for Google products and technologies |
| 8 | [sonnet](https://github.com/google-deepmind/sonnet) | 9918 | active | TensorFlow-based neural network library |
| 9 | [adk-samples](https://github.com/google/adk-samples) | 9366 | active | A collection of sample agents built with Agent Development Kit (ADK) |
| 10 | [trax](https://github.com/google/trax) | 8304 | `archived` | Trax — Deep Learning with Clear Code and Speed |
| 11 | [pysc2](https://github.com/google-deepmind/pysc2) | 8292 | active | StarCraft II Learning Environment |
| 12 | [diff-match-patch](https://github.com/google/diff-match-patch) | 8118 | `archived` | Diff Match Patch is a high-performance library in multiple languages that manipulates plain te… |
| 13 | [alphafold3](https://github.com/google-deepmind/alphafold3) | 8054 | active | AlphaFold 3 inference pipeline. |
| 14 | [latexify_py](https://github.com/google/latexify_py) | 7630 | `archived` | A library to generate LaTeX expression from Python code. |
| 15 | [graphcast](https://github.com/google-deepmind/graphcast) | 6661 | active | - |
| 16 | [gemma_pytorch](https://github.com/google/gemma_pytorch) | 5674 | active | The official PyTorch implementation of Google's Gemma models |
| 17 | [seq2seq](https://github.com/google/seq2seq) | 5627 | `archived` | A general-purpose encoder-decoder framework for Tensorflow |
| 18 | [clusterfuzz](https://github.com/google/clusterfuzz) | 5563 | active | Scalable fuzzing infrastructure. |
| 19 | [graph_nets](https://github.com/google-deepmind/graph_nets) | 5401 | active | Build Graph Nets in Tensorflow |
| 20 | [tf-quant-finance](https://github.com/google/tf-quant-finance) | 5363 | active | High-performance TensorFlow library for quantitative finance. |

---
_This catalog is a point-in-time snapshot; star counts and archive status drift over time. Regenerate with the gh commands above._

