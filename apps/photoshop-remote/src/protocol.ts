// SPDX-License-Identifier: Apache-2.0
//! Adobe Photoshop "Remote Connections" wire protocol (port 49494).
//!
//! 3DES-CBC (des-ede3-cbc), key = PBKDF2-HMAC-SHA1(password, "Adobe Photoshop",
//! 1000, 24 bytes), IV = 8 zero bytes, PKCS#7 padding, NO HMAC. Frame:
//!   [u32 BE length = 4 + ciphertext][u32 BE status=0][ciphertext]
//! cleartext: [u32 BE version=1][u32 BE transaction][u32 BE contentType][body]
//!
//! PBKDF2 uses Bun's native WebCrypto (`crypto.subtle`); 3DES uses node:crypto
//! (WebCrypto has no Triple-DES). Both are Bun-native implementations.

import { createCipheriv, createDecipheriv } from "node:crypto";

export const PS_PORT = 49494;
export const PROTOCOL_VERSION = 1;

const SALT = new TextEncoder().encode("Adobe Photoshop");
const IV = new Uint8Array(8); // always zero
const KEY_BYTES = 24;
const ITERATIONS = 1000;

/** Photoshop message / content types (Adobe `MESSAGE_TYPE_*`). */
export const ContentType = {
  ILLEGAL: 0,
  ERROR_STRING: 1,
  SCRIPT: 2,
  IMAGE: 3,
  PROFILE: 4,
  DATA: 5,
  KEEP_ALIVE: 6,
  FILE_STREAM: 7,
  CANCEL: 8,
  EVENT_STATUS: 9,
  SCRIPT_SHARED: 10,
} as const;

export type ContentTypeValue = (typeof ContentType)[keyof typeof ContentType];

/** A decoded inbound message. */
export interface PsMessage {
  status: number;
  protocol: number;
  transaction: number;
  contentType: number;
  body: Buffer;
}

/** Derive the 24-byte 3DES key via Bun-native WebCrypto PBKDF2-SHA1. */
export async function deriveKey(password: string): Promise<Buffer> {
  const base = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(password),
    "PBKDF2",
    false,
    ["deriveBits"],
  );
  const bits = await crypto.subtle.deriveBits(
    { name: "PBKDF2", hash: "SHA-1", salt: SALT, iterations: ITERATIONS },
    base,
    KEY_BYTES * 8,
  );
  return Buffer.from(bits);
}

function encrypt(key: Buffer, data: Buffer): Buffer {
  const c = createCipheriv("des-ede3-cbc", key, IV); // PKCS#7 auto-pad
  return Buffer.concat([c.update(data), c.final()]);
}

function decrypt(key: Buffer, data: Buffer): Buffer {
  const d = createDecipheriv("des-ede3-cbc", key, IV);
  return Buffer.concat([d.update(data), d.final()]);
}

/** Build an outbound frame carrying `code` as the given content type. */
export function frame(
  key: Buffer,
  transaction: number,
  code: string,
  contentType: number = ContentType.SCRIPT,
): Buffer {
  const head = Buffer.alloc(12);
  head.writeUInt32BE(PROTOCOL_VERSION, 0);
  head.writeUInt32BE(transaction, 4);
  head.writeUInt32BE(contentType, 8);
  const ct = encrypt(key, Buffer.concat([head, Buffer.from(code, "utf8")]));
  const out = Buffer.alloc(8 + ct.length);
  out.writeUInt32BE(4 + ct.length, 0); // length = status(4) + ciphertext
  out.writeUInt32BE(0, 4); // communication status: no error
  ct.copy(out, 8);
  return out;
}

/**
 * Decode one frame's `body` (the bytes after the 4-byte length prefix). A
 * non-zero communication status (e.g. wrong password) is returned as-is with
 * empty body rather than throwing here.
 */
export function parse(key: Buffer, body: Buffer): PsMessage {
  const status = body.readUInt32BE(0);
  if (status !== 0) {
    return { status, protocol: 0, transaction: 0, contentType: 0, body: Buffer.alloc(0) };
  }
  const pt = decrypt(key, body.subarray(4));
  return {
    status,
    protocol: pt.readUInt32BE(0),
    transaction: pt.readUInt32BE(4),
    contentType: pt.readUInt32BE(8),
    body: pt.subarray(12),
  };
}
