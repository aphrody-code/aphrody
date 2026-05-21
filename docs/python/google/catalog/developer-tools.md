# Google · Developer Tools

Google's Python developer-tool releases span code formatting and static analysis, hardware and E2E test frameworks, CLI scaffolding, AST manipulation, configuration/dependency-injection libraries, and a set of eclectic Python utilities maintained at Google scale.

> Part of [`docs/python/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 50 repos (35 active / 15 archived).

---

## CLI & File-Type Detection

### [python-fire](https://github.com/google/python-fire)
**★ 28193 · `active` · pushed 2026-04 · NOASSERTION**
Topics: `cli` `python`

Python Fire automatically generates a fully functional CLI from any Python object — function, class, module, dict, or list — by calling `fire.Fire()` on it. No argument parser boilerplate is required; Fire introspects the object's signature and docstring to build help text, tab-completion data, and subcommand trees. It also ships a lightweight REPL mode useful for debugging live objects.

### [magika](https://github.com/google/magika)
**★ 17033 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `ai` `deep-learning` `filetype` `keras-classification-models` `mime-types` `onnx`

Magika is an AI-powered file-content-type detector backed by a custom, highly optimized Keras/ONNX model weighing only a few MB. It achieves ~99% average precision and recall across 200+ content types on a 100M-sample test set, running inference in ~5 ms per file on a single CPU regardless of file size. Magika is used in production at Google to route Gmail, Drive, and Safe Browsing files to the appropriate security scanners, processing hundreds of billions of samples weekly; it is also integrated with VirusTotal and abuse.ch. The Python API, a Rust CLI binary, and JavaScript/TypeScript bindings are all open source.

---

## Formatters & Linters

### [yapf](https://github.com/google/yapf)
**★ 13984 · `active` · pushed 2026-04 · Apache-2.0**
Topics: `formatter` `google` `python`

YAPF (Yet Another Python Formatter) is a reformatter based on the same algorithm as `clang-format`: it computes the optimal line breaks according to a configured style guide rather than simply enforcing a set of rules. Supported styles include `pep8`, `google`, `facebook`, and `chromium`. Unlike autopep8, YAPF rewrites entire files to match the target style, not just lines that violate it. It is installable via `pip install yapf` and integrates with most editors through pre-commit hooks.

### [pytype](https://github.com/google/pytype)
**★ 5038 · `active` · pushed 2026-03 · NOASSERTION**
Topics: `linter` `python` `static-analysis` `static-code-analysis` `typechecker` `types` `typing`

Pytype is Google's static type analyzer for Python; it infers types from code flow rather than requiring complete annotations, and can generate stub files (`.pyi`). Development started in 2012 and pytype collaborated with Guido van Rossum and the mypy team to create typeshed. The team has announced that Python 3.12 will be the last supported version as they shift investment toward new typing approaches; users are encouraged to evaluate mature alternatives such as mypy, pyright, and pyre. The project remains active for maintenance.

---

## Test Frameworks

### [mobly](https://github.com/google/mobly)
**★ 740 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `android` `android-test` `automation` `bluetooth` `mobile` `networking` `python` `robotics` `telephony` `test-automation` `testing` `wifi` `windows`

Mobly is a Python test framework purpose-built for multi-device or complex-environment test scenarios: P2P data transfer, conference calls across several phones, wearable/phone interactions, IoT device meshes, RF characterization with specialist equipment, and LTE network tests involving phones and base stations simultaneously. It ships controller libraries for Android devices (via ADB), SSH targets, and arbitrary custom hardware, all addressable through a unified YAML configuration. Requires Python 3.11 or newer and runs on Linux, macOS, and Windows.

### [openhtf](https://github.com/google/openhtf)
**★ 680 · `active` · pushed 2026-05 · Apache-2.0**

OpenHTF (Open Hardware Testing Framework) removes boilerplate from hardware-in-the-loop test programs. Test engineers define phases (individual test steps), compose them into test plans, and OpenHTF handles execution flow, pass/fail verdicts, measurement logging, and a browser-based live status UI. It is designed to work from lab bench through manufacturing floor, and is general enough to cover non-hardware scenarios such as software integration tests with physical devices. Installable via `pip install openhtf`.

### [gtest-parallel](https://github.com/google/gtest-parallel)
**★ 469 · `active` · pushed 2025-07 · Apache-2.0**

A Python script that wraps Google Test (gtest) binary execution to run individual test cases in parallel across multiple worker processes. It reads the test list from the gtest binary, shards it across workers, and aggregates XML reports. Useful for cutting wall-clock time of large C++ test suites in CI without modifying the tests themselves.

---

## Configuration & Dependency Injection

### [gin-config](https://github.com/google/gin-config)
**★ 2152 · `active` · pushed 2026-04 · Apache-2.0**
Topics: `configuration-management` `python` `tensorflow` `tensorflow-experiments`

Gin provides a lightweight, Python-first configuration framework based on dependency injection. Functions and classes decorated with `@gin.configurable` expose their parameters to a simple text config syntax that can be supplied from files or the command line. This eliminates the need for large protobuf config objects or manual parameter plumbing. Gin is particularly suited for ML experiments (TensorFlow, JAX) where hyperparameters are deeply nested and subject to frequent change.

### [fiddle](https://github.com/google/fiddle)
**★ 382 · `active` · pushed 2026-05 · Apache-2.0**

Fiddle is a Python-first configuration library aimed at ML applications. It represents configurations as typed, inspectable `fdl.Config` and `fdl.Partial` objects backed by the actual constructors they configure, making the config graph navigable and diffable in Python code rather than in text files. Compared to Gin, Fiddle favors explicit Python expressions over a custom config DSL. Install via `pip install fiddle`; documentation at `fiddle.readthedocs.io`.

### [pinject](https://github.com/google/pinject)
**★ 1344 · `archived` · pushed 2021-03 · Apache-2.0**

Pinject is a Pythonic dependency injection library that wires objects together by inspecting constructor argument names and matching them against registered bindings. It eliminates manual object construction trees while keeping bindings explicit. The library is archived; users requiring active maintenance should evaluate alternatives. Was available as `pip install pinject`.

---

## AST Manipulation & Compilers

### [pyglove](https://github.com/google/pyglove)
**★ 711 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `automl` `evolution` `machine-learning` `manipulation` `meta-learning` `meta-programming` `python` `symbolic-programming`

PyGlove introduces symbolic object-oriented programming to Python: objects become mutable symbolic nodes whose structure can be queried, traversed, and patched at runtime. This enables AutoML workflows where search algorithms treat Python programs as first-class data. PyGlove provides a mutable symbolic object model, a rich operation set for program manipulation, search-space primitives, and a library of search algorithms including evolutionary and random strategies. It is lightweight and has minimal dependencies beyond the standard library.

### [pasta](https://github.com/google/pasta)
**★ 359 · `active` · pushed 2025-03 · Apache-2.0**

Pasta is a library for source-to-source transformation of Python code via AST manipulation that preserves the original formatting wherever possible. Rather than unparsing the whole tree (which would discard comments and whitespace), pasta tracks the original source positions of each node and splices in only the modified fragments. It is used by pytype to emit precise edits. Install via `pip install pasta`.

### [tangent](https://github.com/google/tangent)
**★ 2322 · `archived` · pushed 2022-09 · Apache-2.0**

Tangent performs source-to-source automatic differentiation of Python/NumPy programs, producing human-readable derivative code as a new Python function in the same source file. Unlike operator-overloading AD (e.g., PyTorch autograd), Tangent's output is debuggable with standard Python tools. The project is archived; modern alternatives include JAX.

### [latexify_py](https://github.com/google/latexify_py)
**★ 7630 · `archived` · pushed 2025-02 · Apache-2.0**

`latexify` generates LaTeX mathematical expressions from Python source code by parsing the function's AST and mapping arithmetic operations, function calls, and control flow to their LaTeX equivalents. A decorator `@latexify.function` makes Jupyter notebooks render functions as formulas. The repo is archived; users may fork it.

---

## Parsing & Text Processing

### [textfsm](https://github.com/google/textfsm)
**★ 1234 · `active` · pushed 2025-04 · Apache-2.0**

TextFSM parses semi-structured text (such as CLI output from network devices) into Python tables using a template language that describes state machines. Each template defines a set of value-capturing rules and state transitions; TextFSM drives the FSM over the input, collecting structured records. It is widely used in network automation tooling (ntc-templates ships hundreds of device templates built on TextFSM).

### [diff-match-patch](https://github.com/google/diff-match-patch)
**★ 8118 · `archived` · pushed 2024-05 · Apache-2.0**
Topics: `diff` `difference` `match` `patch` `text-processing`

Diff Match Patch implements Myers diff, Bitap fuzzy match, and patch application algorithms across Python, Java, JavaScript, C++, C#, Lua, Ruby, and Dart. The Python package is archived but remains widely imported in existing codebases. The algorithm produces minimal diffs and can apply patches to near-matching targets with configurable tolerance.

---

## Python Utilities

### [etils](https://github.com/google/etils)
**★ 256 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `jax` `numpy` `python` `tensorflow` `utils`

etils (eclectic utils) is a collection of independent Python utility submodules, each with its own Bazel build rule and minimal dependency footprint. Key modules include `epath` (pathlib-compatible API for `gs://` and `s3://` paths), `enp` (NumPy array utilities), `ejax` (JAX helpers), `ecolab` (Colab/Jupyter utilities), `edc` (dataclass extras), and `eapp` (Abseil flags utilities). Modules are namespaced with an `e` prefix to avoid collisions. Documentation at `etils.readthedocs.io`.

### [vimdoc](https://github.com/google/vimdoc)
**★ 305 · `active` · pushed 2024-09 · Apache-2.0**
Topics: `vim`

vimdoc generates Vim helpfile documentation from specially formatted comments in Vim script files, following a convention similar to Javadoc. It supports `@usage`, `@param`, `@returns`, and `@section` annotations and produces properly indexed `.txt` helpfiles that integrate with Vim's `:help` system.

### [vroom](https://github.com/google/vroom)
**★ 274 · `active` · pushed 2025-06 · Apache-2.0**

vroom is a test runner for Vim scripts: it parses `.vroom` files (plain-text scripts interleaved with expected output) and drives a Vim instance to execute the commands, then asserts that the output matches. It enables regression testing of Vim plugins without a browser or external test harness.

---

## Build & Packaging

### [subpar](https://github.com/google/subpar)
**★ 570 · `archived` · pushed 2022-12 · Apache-2.0**

Subpar creates self-contained Python executable archives (`.par` files, analogous to `.pex`) designed to integrate with Bazel. It bundles a Python application and all its transitive dependencies into a single ZIP-based executable that can be copied and run on any compatible system without a separate installation step. The project is archived.

### [bazel-to-cmake](https://github.com/google/bazel-to-cmake)
**★ 204 · `archived` · pushed 2022-07 · Apache-2.0**

A Python script that translates a subset of Bazel `BUILD` files to equivalent `CMakeLists.txt` entries, intended to help projects that need to support both build systems. Handles common rules such as `cc_library`, `cc_binary`, and `cc_test`. The project is archived.

---

## Debugging

### [pyringe](https://github.com/google/pyringe)
**★ 1629 · `archived` · pushed 2019-12 · NOASSERTION**

pyringe attaches to running CPython processes without prior instrumentation and can inject arbitrary Python code into them. It exposes the attached process's interpreter state, allowing inspection of live objects, stack frames, and thread locals. The project is archived and targets CPython 2.x; for modern Python the `py-spy` and `gdb`-based approaches are preferred.

### [gdb_gcore_point](https://github.com/google/gdb_gcore_point)
**★ 2 · `active` · pushed 2025-02 · Apache-2.0**

A GDB Python script that adds a new breakpoint type (`gcore_point`) which automatically captures a core dump via `gcore` each time the breakpoint is hit, rather than stopping execution. Useful for capturing transient states in long-running processes without halting the program.

---

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [gif-for-cli](https://github.com/google/gif-for-cli) | 2951 | archived | Render animated GIFs in a terminal using ANSI escape codes |
| [enjarify](https://github.com/google/enjarify) | 2748 | archived | Translate Dalvik bytecode to equivalent Java bytecode (`.jar`) |
| [atheris](https://github.com/google/atheris) | 1624 | active | Coverage-guided Python fuzzer backed by libFuzzer (see security catalog) |
| [digitalbuildings](https://github.com/google/digitalbuildings) | 444 | active | Building ontology and SDK used internally by Google for smart-building management |
| [swift-jupyter](https://github.com/google/swift-jupyter) | 624 | archived | Swift kernel for Jupyter notebooks |
| [pybadges](https://github.com/google/pybadges) | 496 | archived | Python library for generating GitHub-style SVG badges |
| [importlab](https://github.com/google/importlab) | 179 | archived | Infers import dependency graphs for Python files; used by pytype |
| [compynator](https://github.com/google/compynator) | 91 | archived | Pure-Python parser combinator library with asymptotically optimal performance |
| [tmppy](https://github.com/google/tmppy) | 34 | archived | Compile a Python subset (TMPPy) to C++ template metafunctions |
| [dpy](https://github.com/google/dpy) | 63 | archived | Python inversion-of-control / dependency injection (older predecessor to pinject) |
| [closure-linter](https://github.com/google/closure-linter) | 113 | archived | JavaScript Closure linter, exported from code.google.com |
| [rfmt](https://github.com/google/rfmt) | 86 | archived | R source code formatter |
| [pytypedecl](https://github.com/google/pytypedecl) | 66 | archived | Early prototype of Python type declaration syntax (predates PEP 484) |
| [anvil-build](https://github.com/google/anvil-build) | 58 | archived | Parallel build system and content pipeline |
| [binplist](https://github.com/google/binplist) | 54 | archived | Binary property-list (`.bplist`) parser in Python |
| [py-ast-utils](https://github.com/google/py-ast-utils) | 18 | archived | Small utilities for Python AST analysis |
| [gazoo-device](https://github.com/google/gazoo-device) | 28 | active | Device manager framework for smart-device testing (GDM) with CLI |
| [mobly-wifi](https://github.com/google/mobly-wifi) | 8 | active | Mobly controller module for Wi-Fi devices |
| [mobly-cros](https://github.com/google/mobly-cros) | 2 | active | Mobly controller module for ChromeOS devices |
| [mobly-windows](https://github.com/google/mobly-windows) | 4 | archived | Mobly controller module for Windows devices |
| [fhir-py](https://github.com/google/fhir-py) | 101 | active | Python utilities for FHIR, including flat-view builders for BigQuery |
| [terminal-app-function-keys](https://github.com/google/terminal-app-function-keys) | 91 | archived | macOS Terminal.app config for correct Fn-key handling |
| [gfw-toolkit](https://github.com/google/gfw-toolkit) | 29 | archived | CLI scripts for Google Apps Admin SDK |
| [ldpush](https://github.com/google/ldpush) | 37 | archived | Cross-vendor network device configuration distribution tool |
| [python-yaml-config](https://github.com/google/python-yaml-config) | 14 | archived | Minimal YAML-based Python config library |
| [smilesparser](https://github.com/google/smilesparser) | 15 | archived | SMILES chemical notation parser |
| [viai-sdk](https://github.com/google/viai-sdk) | 6 | archived | SDK for Google Cloud Visual Inspection AI |
