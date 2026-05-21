<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# Go in the aphrody corpus

aphrody is a 100% Rust project (see [`CLAUDE.md`](../../CLAUDE.md) §2). It ships
**no** Go code in its build. Go shows up here in exactly two ways:

1. **Reverse-engineering intel** — the proprietary binaries aphrody talks to (or
   has analysed) are written in Go. The largest is the ~136 MB Codeium-derived
   `language_server.exe` that powers Google's Antigravity IDE; a sibling Go
   binary, `localharness`, ships in the `google-antigravity` Python SDK. aphrody
   does **not** vendor or compile these — it reproduces their *launch / wire
   contract* in Rust so it can interoperate without the proprietary shell.
2. **Style reference** — the Google Go Style Guide was imported into the repo
   (commit `976b998f2`, "feat: set up Go and import Google Go Style Guide") as a
   readability reference. Go 1.26.3 was installed on the host at the same time
   (`go version go1.26.3 windows/amd64`); this is tooling, not a build target.

This directory documents both. **No secrets, tokens, or cookie values appear in
any of these files** — only public RPC names, field shapes, launch flags, and
public certificate fingerprints. Personal paths are anonymised to `<user>`.

## Index

| Document | Scope |
|---|---|
| [`antigravity-language-server.md`](antigravity-language-server.md) | Deep dive on the Go `language_server.exe`: the `exa.language_server_pb` gRPC service surface, the launch contract, and how the Rust `local_ls` bridge talks to it. |
| [`google-go-style-guide.md`](google-go-style-guide.md) | Distilled summary of the imported Google Go Style Guide (key rules + where the source lives in-repo). |
| [`go-binaries-inventory.md`](go-binaries-inventory.md) | Table of every Go binary/artifact found in the corpus: path, size, role, and how aphrody interacts with it. |

The pre-existing files in this directory — [`index.md`](index.md),
[`guide.md`](guide.md), [`decisions.md`](decisions.md),
[`best-practices.md`](best-practices.md), and [`ARCHITECTURE.md`](ARCHITECTURE.md)
— are the verbatim imported Google Go Style Guide pages (Markdown conversions of
<https://google.github.io/styleguide/go>) plus the in-repo architecture note.
This README and the three new files above are the RE-intel layer on top of them.

## Where Go appears (summary)

| Surface | Language | Go? | Evidence |
|---|---|---|---|
| Antigravity engine `language_server.exe` (~136 MB) | Go | **Yes** | `codeium_common_go_proto` symbols, `exa.*_pb` gRPC; [`docs/research/antigravity-sdk-analysis.md`](../research/antigravity-sdk-analysis.md) §0.0, `var/data/antigravity-extract/REPORT.md` (local, gitignored) |
| `localharness` (google-antigravity Python SDK harness) | Go | **Yes** | "pre-compiled Go binary in the wheel", [`python/antigravity-sdk-python/pyproject.toml`](../../python/antigravity-sdk-python/pyproject.toml) lines 67-71 |
| aphrody `crates/antigravity-sdk` | Rust | No (talks *to* Go) | [`crates/antigravity-sdk/src/local_ls.rs`](../../crates/antigravity-sdk/src/local_ls.rs) |
| bxc reference (`var/data/bxc-ref/`) | Bun / TypeScript / Zig / Rust | **No** | `package.json` `packageManager: bun@1.3.14`, no `go.mod`, no `*.go` (confirmed by scan) |
| aphrody workspace (71 crates) | Rust | No | [`CLAUDE.md`](../../CLAUDE.md) §2 |

## External facts (public)

- The Codeium / Windsurf language server is a Go binary
  (`language_server_linux_x64` etc.) communicating with the cloud over gRPC +
  protobuf — see Exafunction/codeium issues
  <https://github.com/Exafunction/codeium/issues/285> and
  <https://github.com/Exafunction/codeium/issues/286>, and the Windsurf plugin
  docs <https://docs.windsurf.com/plugins/getting-started>.
- gRPC for Go is `google.golang.org/grpc` (<https://github.com/grpc/grpc-go>);
  protobuf for Go is `google.golang.org/protobuf`; codegen via `protoc-gen-go`
  and `protoc-gen-go-grpc`
  (<https://pkg.go.dev/google.golang.org/grpc/cmd/protoc-gen-go-grpc>,
  <https://grpc.io/docs/languages/go/quickstart/>). The `_pb` package suffix and
  the `codeium_common_go_proto` symbol naming are consistent with this toolchain.
- Google publishes its Go Style Guide at
  <https://google.github.io/styleguide/go>.
