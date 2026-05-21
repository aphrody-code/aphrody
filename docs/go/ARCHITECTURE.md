# Go Architecture — aphrody

This document describes the Go-based sub-systems within `aphrody`.

Currently, Go is leveraged to handle two key performance-sensitive tasks:
1. **Exact BPE (Byte-Pair Encoding) tokenization** (`crates/aphrody-tokenizer-go`) without bloating the core Rust binary with heavy tokenization tables or Python runtime requirements.
2. **HTML-to-text/markdown-like parsing and extraction** leveraging Google's official HTML parser sub-repository (`golang.org/x/net/html`) to clean raw web page context.

---

## 1. Bird's-Eye View

The Go tokenizer operates as a decoupled companion executable (`aphrody-tokenizer-go`). The core Rust application interacts with it via process spawning and standard input/output (IPC).

```mermaid
graph TD
    Rust[Rust Context Engine] -- Spawns / IPC --> GoBin[Go: aphrody-tokenizer-go]
    GoBin -- Reads JSON command from Stdin --> Dispatcher{Command Dispatcher}
    Dispatcher -- BPE Count --> Tokenizer[tiktoken-go Library]
    Dispatcher -- HTML-to-Text --> HTMLParser[golang.org/x/net/html]
    Tokenizer -- Tokens --> GoBin
    HTMLParser -- Cleaned Markdown Text --> GoBin
    GoBin -- Writes JSON response to Stdout --> Rust
```

---

## 2. IPC Protocol & JSON Schemas

Communication uses a line-oriented or single-shot JSON exchange via Standard Input (`stdin`) and Standard Output (`stdout`).

### 2.1 BPE Token Counting Schema

#### Stdin Request Schema
```json
{
  "encoding": "cl100k_base",
  "text": "hello world"
}
```

* **`encoding`** (optional, defaults to `cl100k_base`): The target BPE encoding to use. Currently supported:
  * `cl100k_base` / `cl100k` (Claude 3 / GPT-4)
  * `o200k_base` / `o200k` (GPT-4o)
  * `p50k_base` / `p50k` (Codex)
  * `r50k_base` / `r50k` / `gpt2` (GPT-2 / GPT-3)
* **`text`**: The input string to tokenize.

#### Stdout Response Schema
On success:
```json
{
  "tokens": 2
}
```

On failure:
```json
{
  "error": "unknown encoding: invalid_name"
}
```

---

### 2.2 HTML-to-Text Parser Schema

#### Stdin Request Schema
```json
{
  "command": "html2text",
  "html": "<h1>My Header</h1><p>Check out <a href='https://example.com'>this link</a>.</p>"
}
```

* **`command`**: Must be set to `"html2text"`.
* **`html`**: The raw HTML string to be stripped and formatted.

#### Stdout Response Schema
On success:
```json
{
  "text": "# My Header\n\nCheck out [this link](https://example.com)."
}
```

On failure:
```json
{
  "error": "failed to parse HTML: <reason>"
}
```

---

## 3. Rust-to-Go IPC Bridge

Inside the `aphrody-context` crate, the `GoTokenEstimator` manages the interaction:

1. **Resolution**: The Rust coordinator resolves the compiled Go binary path using multiple search steps (Env override `APHRODY_TOKENIZER_GO_BIN`, sibling binary location, system `PATH` walk, and repository parent lookup).
2. **Process Spawn**: The binary is executed with piped stdin/stdout and silent stderr.
3. **JSON Serialization**: The Rust estimator writes the JSON payload representing either a tokenization or extraction query to stdin.
4. **Fallback Handling**: If spawning, piping, writing, or reading fails (or if Go returns an error object), the estimator gracefully falls back to the local `HeuristicTokenEstimator` to prevent crashing the host process.

---

## 4. Compilation & Deployment

The Go binary is compiled target-native:
* On Windows: `aphrody-tokenizer-go.exe`
* On Linux/macOS: `aphrody-tokenizer-go`

During development or verification tests, the binary can be compiled locally:
```bash
cd crates/aphrody-tokenizer-go
go build -o aphrody-tokenizer-go.exe main.go
```

The compiled binary is placed adjacent to the running binary or in `crates/aphrody-tokenizer-go/` to allow automated tests to automatically resolve it.

### CLI Manual Verification

You can verify the subcommands manually from the command line:

```bash
# Test exact token count
.\aphrody-tokenizer-go.exe count cl100k_base "Hello, world!"
# Output: 4

# Test HTML-to-text parsing via inline args
.\aphrody-tokenizer-go.exe html2text "<h1>Title</h1><p>Text</p>"
# Output:
# # Title
#
# Text

# Test HTML-to-text parsing via stdin pipe
echo "<div>Hello <span>world</span></div>" | .\aphrody-tokenizer-go.exe html2text
# Output: Hello world
```

---

## 5. Style Decisions & Guidelines

Go development in `aphrody` adheres strictly to the **Google Go Style Guide** (documented in [guide.md](guide.md) and [decisions.md](decisions.md)):

* **Clarity & Simplicity**: Keep the binary specialized. Avoid building extra layers, databases, or networking inside the tokenizer companion.
* **Errors over Panics**: Never panic. All errors (unrecognized encoding, serialization issues, empty values) are captured and returned cleanly via JSON or printed to `stderr` with non-zero exit codes.
* **Google Sub-repositories Integration**: For low-level HTML tokenization, the project utilizes the official Google-maintained sub-repository `golang.org/x/net/html` to ensure performance, compliance, and standard formatting.
* **No AI trailers or leaks**: Avoid AI comments/fingerprints. Maintain SPDX-License headers on all new source files.

