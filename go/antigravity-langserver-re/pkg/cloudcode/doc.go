// SPDX-License-Identifier: Apache-2.0

// Package cloudcode reconstructs the Cloud Code Private API ("v1internal")
// surface that the Antigravity language server calls as its remote backend.
//
// Provenance (RE — not official source):
//   - google3/google/internal/cloud/code/v1internal/v1internal_jetski_service_go_proto
//     -> JetskiService (gRPC), exposed over REST as v1internal:<method>.
//   - google3/google/internal/cloud/code/v1internal/v1internal_prediction_service_go_proto
//     -> PredictionService (gRPC) for the generation/quota surface.
//   - google3/google/internal/cloud/code/v1internal/{v1internal,credits,remote_context}_go_proto
//
// Method names recovered verbatim from redress source projection of the
// Unimplemented*ServiceServer-fm thunks. Message type names and field names
// recovered from the proto Get<Field> accessor methods. Field *types* are
// inferred (proto wire types are not encoded in the recovered symbols).
//
// Hosts (from ls-strings.txt):
//   - https://cloudcode-pa.googleapis.com         (primary)
//   - https://daily-cloudcode-pa.googleapis.com   (daily/runtime variant)
//   - https://aiplatform.googleapis.com           (Vertex; google + anthropic publishers)
//   - aicode.googleapis.com:443                    (dedicated gRPC)
package cloudcode
