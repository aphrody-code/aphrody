// SPDX-License-Identifier: Apache-2.0
//! `PhotoshopRemote` — a Bun-native client for Photoshop's Remote Connections.
//!
//! Uses `Bun.connect` (native TCP) and the protocol in `./protocol`. Each
//! `exec()` sends an ExtendScript and resolves with the correlated response,
//! matched by transaction id over a single multiplexed socket.

import type { Socket } from "bun";
import {
  ContentType,
  PS_PORT,
  PsMessage,
  deriveKey,
  frame,
  parse,
} from "./protocol";

export interface PhotoshopRemoteOptions {
  host: string;
  password: string;
  port?: number;
}

export interface ExecResult {
  transaction: number;
  contentType: number;
  /** UTF-8 body (for SCRIPT results / error strings). */
  text: string;
  /** True when Photoshop returned an ERROR_STRING or a comm-status error. */
  isError: boolean;
}

interface Pending {
  resolve: (m: PsMessage) => void;
  reject: (e: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class PhotoshopRemote {
  readonly #opts: Required<PhotoshopRemoteOptions>;
  #socket: Socket | null = null;
  #key: Buffer | null = null;
  #buf: Buffer = Buffer.alloc(0);
  #tid = 0;
  readonly #pending = new Map<number, Pending>();

  constructor(opts: PhotoshopRemoteOptions) {
    this.#opts = { port: PS_PORT, ...opts };
  }

  /** Derive the key and open the TCP socket (`Bun.connect`). */
  async connect(): Promise<void> {
    this.#key = await deriveKey(this.#opts.password);
    this.#socket = await Bun.connect({
      hostname: this.#opts.host,
      port: this.#opts.port,
      socket: {
        data: (_s, chunk) => this.#onData(chunk),
        close: () => this.#failAll(new Error("socket closed")),
        error: (_s, err) =>
          this.#failAll(err instanceof Error ? err : new Error(String(err))),
      },
    });
  }

  #onData(chunk: Uint8Array): void {
    this.#buf = Buffer.concat([this.#buf, Buffer.from(chunk)]);
    while (this.#buf.length >= 4) {
      const len = this.#buf.readUInt32BE(0);
      if (this.#buf.length < 4 + len) break;
      const body = Buffer.from(this.#buf.subarray(4, 4 + len));
      this.#buf = this.#buf.subarray(4 + len);
      let msg: PsMessage;
      try {
        msg = parse(this.#key!, body);
      } catch {
        this.#failAll(new Error("decrypt/unpad failed (wrong password?)"));
        return;
      }
      if (msg.status !== 0) {
        this.#failAll(new Error(`communication status ${msg.status} (wrong password?)`));
        return;
      }
      const waiter = this.#pending.get(msg.transaction);
      if (waiter) {
        clearTimeout(waiter.timer);
        this.#pending.delete(msg.transaction);
        waiter.resolve(msg);
      }
    }
  }

  /**
   * Run an ExtendScript string inside the remote Photoshop. `shared` uses the
   * shared scripting engine (content type 10) instead of a fresh one (2).
   */
  exec(
    jsx: string,
    opts: { timeoutMs?: number; shared?: boolean } = {},
  ): Promise<ExecResult> {
    if (!this.#socket || !this.#key) {
      return Promise.reject(new Error("not connected — call connect() first"));
    }
    const tid = this.#nextTid();
    const type = opts.shared ? ContentType.SCRIPT_SHARED : ContentType.SCRIPT;
    const payload = frame(this.#key, tid, jsx, type);
    return new Promise<ExecResult>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(tid);
        reject(new Error("timeout waiting for Photoshop response"));
      }, opts.timeoutMs ?? 30_000);
      this.#pending.set(tid, {
        reject,
        timer,
        resolve: (m) =>
          resolve({
            transaction: m.transaction,
            contentType: m.contentType,
            text: m.body.toString("utf8"),
            isError: m.contentType === ContentType.ERROR_STRING,
          }),
      });
      this.#socket!.write(payload);
    });
  }

  /** Convenience: app version + open-document count + active document name. */
  async info(): Promise<ExecResult> {
    return this.exec(
      'var n = app.documents.length; var a = n > 0 ? app.activeDocument.name : "(none)";' +
        ' "PS " + app.version + " | docs=" + n + " | active=" + a;',
    );
  }

  close(): void {
    this.#socket?.end();
    this.#socket = null;
    this.#failAll(new Error("client closed"));
  }

  #nextTid(): number {
    this.#tid = (this.#tid + 1) % 0xff_ff_ff;
    if (this.#tid === 0) this.#tid = 1;
    return this.#tid;
  }

  #failAll(err: Error): void {
    for (const w of this.#pending.values()) {
      clearTimeout(w.timer);
      w.reject(err);
    }
    this.#pending.clear();
  }
}
