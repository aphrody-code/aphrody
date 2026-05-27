// SPDX-License-Identifier: Apache-2.0

// Package cascade reconstructs the CascadeManager — the agentic execution loop
// (engine name "Cascade", inherited from Windsurf) that the language server
// drives behind LanguageServerService.
//
// Provenance (RE — not official source):
//   - google3/third_party/jetski/cortex/cortex cascade_manager.go and
//     battlemode.go -> CascadeManager method set, recovered verbatim from the
//     pclntab via redress source projection.
//
// Method bodies are not recoverable from the stripped binary; the manager is
// reconstructed as an interface plus a documented struct so the agent lifecycle
// is captured faithfully (method names) without inventing behaviour.
package cascade
