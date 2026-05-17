// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// env.ts — typed, validating access to Bun.env / process.env.
//
// n2b flags `api/process-env` hundreds of times on average Node→Bun
// migrations (173 on rpb-dashboard alone) because callers either parse
// env inline (error-prone) or ship ad-hoc defaults. This module gives
// them a safe, typed accessor with required/default/parse semantics,
// backed by Bun.env (which is strictly faster than Node's process.env).

export class EnvError extends Error {
  constructor(public key: string, reason: string) {
    super(`env: ${key} — ${reason}`);
    this.name = "EnvError";
  }
}

export interface EnvOptions<T> {
  /** Required: throw EnvError if the var is absent or empty. */
  required?: boolean;
  /** Default value when var is absent. Ignored if `required`. */
  default?: T;
  /** Optional parser. Raw string → T. Exceptions are wrapped in EnvError. */
  parse?: (raw: string) => T;
}

/** Read a string env var with optional required/default. */
export function str(key: string, opts: EnvOptions<string> = {}): string {
  const raw = Bun.env[key];
  if (raw === undefined || raw === "") {
    if (opts.required) throw new EnvError(key, "required but missing");
    return (opts.default ?? "") as string;
  }
  if (opts.parse) {
    try {
      return opts.parse(raw);
    } catch (err) {
      throw new EnvError(key, `parse failed: ${(err as Error).message}`);
    }
  }
  return raw;
}

/** Read an integer env var. Accepts leading + / -, base 10 only. */
export function int(key: string, opts: Omit<EnvOptions<number>, "parse"> = {}): number {
  const raw = Bun.env[key];
  if (raw === undefined || raw === "") {
    if (opts.required) throw new EnvError(key, "required but missing");
    return opts.default ?? 0;
  }
  const n = Number.parseInt(raw, 10);
  if (Number.isNaN(n)) throw new EnvError(key, `not an integer: ${JSON.stringify(raw)}`);
  return n;
}

/** Read a boolean env var. Truthy: 1/true/yes/on ; Falsy: 0/false/no/off. */
export function bool(key: string, opts: Omit<EnvOptions<boolean>, "parse"> = {}): boolean {
  const raw = Bun.env[key]?.toLowerCase();
  if (raw === undefined || raw === "") {
    if (opts.required) throw new EnvError(key, "required but missing");
    return opts.default ?? false;
  }
  if (["1", "true", "yes", "on"].includes(raw)) return true;
  if (["0", "false", "no", "off"].includes(raw)) return false;
  throw new EnvError(key, `not a boolean: ${JSON.stringify(raw)}`);
}

/** Read a URL env var, throws EnvError if malformed. */
export function url(key: string, opts: Omit<EnvOptions<URL>, "parse"> = {}): URL {
  const raw = str(key, { required: opts.required, default: undefined });
  if (raw === "") {
    if (opts.default) return opts.default;
    throw new EnvError(key, "missing");
  }
  try {
    return new URL(raw);
  } catch {
    throw new EnvError(key, `not a valid URL: ${JSON.stringify(raw)}`);
  }
}

/** Read a JSON env var and parse it. */
export function json<T = unknown>(key: string, opts: Omit<EnvOptions<T>, "parse"> = {}): T {
  const raw = Bun.env[key];
  if (raw === undefined || raw === "") {
    if (opts.required) throw new EnvError(key, "required but missing");
    return opts.default as T;
  }
  try {
    return JSON.parse(raw) as T;
  } catch (err) {
    throw new EnvError(key, `JSON parse failed: ${(err as Error).message}`);
  }
}
