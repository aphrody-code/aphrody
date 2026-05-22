// SPDX-License-Identifier: Apache-2.0

// Package langserver reconstructs the local LanguageServerService — the
// Connect/gRPC service the Antigravity VSCode extension and the Jetski webview
// call over the language server's local HTTPS endpoint (self-signed cert.pem).
//
// Provenance (RE — not official source):
//   - google3/third_party/jetski/language_server_pb/language_server_go_grpc
//     -> LanguageServerService server stub (237 RPC methods).
//   - google3/third_party/jetski/language_server_pb/language_server_go_proto_connect
//     -> connect-go transport (LanguageServerServiceHandler*), confirming the
//     service is served over the Connect protocol locally.
//
// Method names recovered verbatim from the Unimplemented*Server-fm and the
// connect Handler-fm thunks via redress source projection. Full method
// signatures (request/response message types) are not encoded in the recovered
// symbols for this build, so the 237 methods are exposed as a documented name
// registry; a small representative subset is modelled as a typed interface.
package langserver
