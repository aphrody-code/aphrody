// SPDX-License-Identifier: Apache-2.0
//! Pure-protocol round-trip tests (no network). `bun test`.

import { describe, expect, test } from "bun:test";
import { ContentType, PROTOCOL_VERSION, deriveKey, frame, parse } from "../src/protocol";

describe("photoshop remote protocol", () => {
  test("PBKDF2-SHA1 derives a 24-byte 3DES key", async () => {
    const key = await deriveKey("password");
    expect(key.length).toBe(24);
    // Deterministic for a fixed password/salt/iterations.
    const again = await deriveKey("password");
    expect(key.equals(again)).toBe(true);
    expect((await deriveKey("other")).equals(key)).toBe(false);
  });

  test("frame → parse round-trips a SCRIPT message", async () => {
    const key = await deriveKey("aphrody");
    const code = 'app.version + " | héllo"; // unicode + symbols';
    const wire = frame(key, 7, code, ContentType.SCRIPT);

    // Frame header: length prefix counts status(4) + ciphertext.
    const len = wire.readUInt32BE(0);
    expect(len).toBe(wire.length - 4);
    expect(wire.readUInt32BE(4)).toBe(0); // comm status

    const msg = parse(key, wire.subarray(4));
    expect(msg.status).toBe(0);
    expect(msg.protocol).toBe(PROTOCOL_VERSION);
    expect(msg.transaction).toBe(7);
    expect(msg.contentType).toBe(ContentType.SCRIPT);
    expect(msg.body.toString("utf8")).toBe(code);
  });

  test("ciphertext length is a multiple of the 3DES block (8)", async () => {
    const key = await deriveKey("aphrody");
    const wire = frame(key, 1, "x");
    const ciphertext = wire.subarray(8);
    expect(ciphertext.length % 8).toBe(0);
  });

  test("wrong key fails to decrypt a frame", async () => {
    const good = await deriveKey("aphrody");
    const bad = await deriveKey("nope");
    const wire = frame(good, 1, "app.version");
    // Decrypting with the wrong key throws (PKCS#7 unpad failure) or yields garbage.
    let threwOrMismatch = false;
    try {
      const msg = parse(bad, wire.subarray(4));
      threwOrMismatch = msg.protocol !== PROTOCOL_VERSION;
    } catch {
      threwOrMismatch = true;
    }
    expect(threwOrMismatch).toBe(true);
  });
});
