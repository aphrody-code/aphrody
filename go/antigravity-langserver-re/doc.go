// SPDX-License-Identifier: Apache-2.0

// Package antigravitylangserverre is a reverse-engineering (RE) reconstruction
// of the Go sidecar shipped with the Antigravity IDE — the binary
//
//	resources/app/extensions/antigravity/bin/language_server_windows_x64.exe
//
// (133 MB, PE64, built with Google-internal toolchain
// "go1.27-20260427-RC04 cl/906595525 +boringcrypto,simd").
//
// # Provenance and honesty
//
// THIS IS NOT OFFICIAL SOURCE. None of the original Google source code was
// available. Every symbol here was recovered from the stripped-but-not-fully
// production binary using:
//
//   - goretk/redress v1.2.67 — pclntab parsing, package tree, per-file/per-type
//     function & method recovery ("source projection"). This is what made the
//     reconstruction possible: redress reads the Go pclntab and recovers, for
//     every package, the file names, the type names, and the *method names*
//     attached to each type (including the proto-generated Get<Field> accessors,
//     which reveal struct field names).
//   - mandiant/GoReSym v1.7.1 — attempted, but FAILED on this binary
//     ("failed to locate pclntab"): GoReSym's pclntab magic table does not yet
//     recognise the unreleased internal go1.27 build. redress (gore lib) does.
//   - Sysinternals strings v2.54 — host/scope/model/clientID constant recovery.
//
// What is FAITHFUL (recovered verbatim from symbols):
//   - the google3 package tree under third_party/jetski/ (209 packages),
//   - service names and their RPC method names
//     (LanguageServerService: 237 methods; v1internal JetskiService: 14;
//     v1internal PredictionService: 5),
//   - proto message *type names* and their field names (from Get<Field> methods),
//   - the auth-provider implementations and their method sets,
//   - the CascadeManager method set (the agentic run loop),
//   - hosts, OAuth scopes, OAuth client IDs and model IDs (from strings).
//
// What is INFERRED (RE could not recover it, so it was reconstructed plausibly):
//   - exact field *types* (proto wire types are not in the symbols — we use Go
//     types consistent with the accessor names and the known Cloud Code REST API),
//   - field ordering and proto field numbers,
//   - method *parameter and return types* (only names are in pclntab, not full
//     signatures for this build), so service interfaces use request/response
//     structs reconstructed from the message type names,
//   - any function body (impossible to recover from a stripped Go binary).
//
// Each reconstructed declaration carries a `// reconstructed from: <symbol>`
// comment pointing at the recovered symbol it is based on.
//
// Raw RE artefacts: C:\src\aphrody\var\data\antigravity-ide-re\
// (redress/source-all.txt, redress/packages-full.txt, ls-strings.txt,
// ls-service-methods.txt, v1internal-methods.txt).
//
// Upstream identity (confirmed by prior RE, docs/research/antigravity-ide-re.md):
// Antigravity is a Google fork of Windsurf / Codeium. Internal codename "Jetski"
// (google3/third_party/jetski/), agent runtime "cortex" (Go), execution engine
// "Cascade" (inherited from Windsurf), proto namespace "exa.*".
package antigravitylangserverre

// BuildInfo records what `redress info` reported for the analysed binary.
//
// reconstructed from: redress info language_server_windows_x64.exe
type BuildInfo struct {
	OS           string // "windows"
	Arch         string // "amd64"
	GoRoot       string // "third_party/go/gc"
	MainRoot     string // "third_party/jetski/cmd/language_server"
	MainPackages int    // 1
	StdPackages  int    // 218
	VendorPkgs   int    // 1300
	Toolchain    string // "go1.27-20260427-RC04 cl/906595525 +boringcrypto,simd"
}

// AnalysedBinary is the fixed BuildInfo for the binary this module reconstructs.
var AnalysedBinary = BuildInfo{
	OS:           "windows",
	Arch:         "amd64",
	GoRoot:       "third_party/go/gc",
	MainRoot:     "third_party/jetski/cmd/language_server",
	MainPackages: 1,
	StdPackages:  218,
	VendorPkgs:   1300,
	Toolchain:    "go1.27-20260427-RC04 cl/906595525 +boringcrypto,simd",
}
