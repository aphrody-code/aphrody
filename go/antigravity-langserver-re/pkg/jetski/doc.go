// SPDX-License-Identifier: Apache-2.0

// Package jetski is the umbrella for the reconstructed "Jetski" tree
// (google3/third_party/jetski/), the Google-internal codename for the
// Antigravity agent. The binary's main package is
// third_party/jetski/cmd/language_server.
//
// This package exposes cross-cutting constants recovered from the binary
// (model IDs, feature-flag hosts, codenames) and re-exports the subpackages:
//   - pkg/cloudcode  — v1internal Cloud Code backend (Jetski/Prediction services)
//   - pkg/langserver — local LanguageServerService (237 RPCs, Connect transport)
//   - pkg/cortex     — cortex agent runtime + tools
//   - pkg/cascade    — CascadeManager agentic run loop
//   - pkg/auth       — OAuth client + auth providers
//
// Provenance: RE reconstruction, not official source. See the module README and
// the top-level doc.go for the full methodology and tool versions.
package jetski

// Codenames recovered from the binary and prior RE.
//
// reconstructed from: package tree (third_party/jetski/), ls-strings.txt
const (
	CodenameJetski  = "jetski"  // Google-internal codename for the agent
	CodenameCortex  = "cortex"  // Go agent runtime package
	CodenameCascade = "Cascade" // execution engine (inherited from Windsurf)
	ProtoNamespace  = "exa"     // Codeium proto namespace (exa.*_pb)
)

// Feature-flag (Unleash) hosts referenced by the agent webview CSP.
//
// reconstructed from: workbench-jetski-agent.html CSP + ls-strings.txt
const (
	UnleashHostJetski      = "http://jetski-unleash.corp.goog/"
	UnleashHostAntigravity = "http://antigravity-unleash.goog/"
)

// ModelIDs are the Gemini/Claude model identifiers recovered from the binary.
// Some carry obvious string-concatenation artefacts in the raw strings dump;
// the cleaned canonical IDs are listed here.
//
// reconstructed from: var/data/antigravity-ide-re/models.txt (ls-strings.txt)
var ModelIDs = []string{
	"gemini-2.5-flash",
	"gemini-2.5-flash-image-preview",
	"gemini-2.5-flash-lite",
	"gemini-2.5-pro",
	"gemini-2.5-pro-windsurf", // Windsurf-origin marker
	"gemini-3-flash-preview",
	"gemini-3-pro-preview",
	"gemini-3.1-flash-image-preview",
	"gemini-3.1-flash-lite-preview",
	"gemini-3.1-pro",
	"gemini-v4s-jarvis", // newly observed in this RE pass
	"claude-haiku-4-5",  // Claude via Vertex (publishers/anthropic)
}
