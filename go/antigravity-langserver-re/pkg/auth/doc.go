// SPDX-License-Identifier: Apache-2.0

// Package auth reconstructs the language-server authentication subsystem.
//
// Provenance (RE — not official source):
//   - google3/third_party/jetski/language_server/auth_client/authclient
//     (auth_client.go) -> AuthClient and its methods.
//   - google3/third_party/jetski/language_server/code_assist_client/codeassistclient
//     (auth_provider.go) -> the AuthProvider interface and 4 implementations:
//     IDEAuthProvider, StandaloneAuthProvider, CLIAuthProvider, AntigravityHubAuthProvider.
//
// OAuth client IDs, scopes and endpoints recovered verbatim from ls-strings.txt.
// Method names recovered from redress source projection. Function bodies are
// not recoverable from a stripped Go binary; method behaviour is documented but
// not reimplemented.
package auth
