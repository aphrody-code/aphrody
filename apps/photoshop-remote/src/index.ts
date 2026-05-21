// SPDX-License-Identifier: Apache-2.0
//! Public surface of the aphrody Photoshop Remote-Connections client.
export { PhotoshopRemote } from "./client";
export type { PhotoshopRemoteOptions, ExecResult } from "./client";
export {
  ContentType,
  PS_PORT,
  PROTOCOL_VERSION,
  deriveKey,
  frame,
  parse,
} from "./protocol";
export type { PsMessage, ContentTypeValue } from "./protocol";
