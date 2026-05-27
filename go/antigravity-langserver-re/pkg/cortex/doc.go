// SPDX-License-Identifier: Apache-2.0

// Package cortex reconstructs the "cortex" agent runtime — the Go package tree
// under google3/third_party/jetski/cortex/ that drives Antigravity's agent
// (the engine inherited from Windsurf is called "Cascade").
//
// Provenance (RE — not official source):
//   - google3/third_party/jetski/cortex/cortex (core: agent_state.go,
//     annotations_manager.go, battlemode.go, cascade_manager.go, client.go,
//     trajectory.go, summaries_store.go, ...).
//   - google3/third_party/jetski/cortex/tools/tools (the agent tool set,
//     recovered as the *ToolConverter symbols).
//   - 70+ further cortex subpackages (handlers, executors, mixins,
//     customizations, sidecars, subagent, sdk, ...) enumerated in
//     var/data/antigravity-ide-re/redress/jetski-packages.txt.
//
// The CascadeManager (the agentic run loop) lives in package cascade.
// Tool/type names recovered from redress source projection; function bodies are
// not recoverable.
package cortex
