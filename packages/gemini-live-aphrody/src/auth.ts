// SPDX-License-Identifier: Apache-2.0
//
// OAuth credential loader for the Gemini CLI's persisted token store.
//
// The Gemini CLI (`gemini auth login`, source at
// gemini-cli/packages/core/src/code_assist/oauth2.ts) writes the user's
// OAuth tokens to a JSON file alongside the rest of its state. We reuse
// that same file so this package never asks the user for a
// `GEMINI_API_KEY`.
//
// File shape (verified against a real credentials file on this machine):
//
//   {
//     "access_token":  string,           // current Bearer token
//     "scope":         string,           // space-separated OAuth scopes
//     "token_type":    "Bearer",
//     "id_token":      string,           // JWT (unused here)
//     "expiry_date":   number,           // ms since epoch (UTC)
//     "refresh_token": string            // long-lived refresh token
//   }
//
// When the cached access_token is expired (or within `EXPIRY_GUARD_MS` of
// expiry) we refresh it against Google's OAuth token endpoint, reusing the
// installed-application client id/secret pair shipped publicly with the
// Gemini CLI. The refreshed token is written back to disk so subsequent
// processes (the CLI itself included) pick up the new value.

import { promises as fs } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// ── Constants ────────────────────────────────────────────────────────────────

/**
 * Default location of the credentials file written by `gemini auth login`.
 * Override with the `GEMINI_CREDENTIALS_PATH` env var.
 */
export const DEFAULT_CREDENTIALS_PATH = join(
  homedir(),
  ".gemini",
  "oauth_creds.json",
);

/**
 * Refresh the token if it expires within this many milliseconds. Mirrors
 * the upstream Gemini CLI's behavior of pre-emptively refreshing rather
 * than waiting for a 401.
 */
const EXPIRY_GUARD_MS = 60_000;

/**
 * Public installed-application OAuth credentials shipped with the Gemini
 * CLI. Per Google's installed-application guidance the "secret" here is
 * not actually a secret (see oauth2.ts in gemini-cli for the upstream
 * comment).
 */
const OAUTH_CLIENT_ID =
  "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const OAUTH_CLIENT_SECRET = "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";
const GOOGLE_TOKEN_URL = "https://oauth2.googleapis.com/token";

// ── Types ────────────────────────────────────────────────────────────────────

/**
 * Shape of the on-disk credentials file as written by the Gemini CLI.
 */
export interface GeminiOAuthCredentials {
  access_token: string;
  refresh_token: string;
  token_type: string;
  scope?: string;
  id_token?: string;
  expiry_date: number;
}

/**
 * Response payload from Google's `/token` endpoint when refreshing.
 */
interface GoogleRefreshResponse {
  access_token: string;
  expires_in: number;
  scope?: string;
  token_type: string;
  id_token?: string;
}

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * Resolve the credentials path, honoring `GEMINI_CREDENTIALS_PATH`.
 */
export function resolveCredentialsPath(): string {
  const override = process.env["GEMINI_CREDENTIALS_PATH"];
  if (override && override.length > 0) {
    return override;
  }
  return DEFAULT_CREDENTIALS_PATH;
}

/**
 * Read and parse the OAuth credentials file written by the Gemini CLI.
 *
 * Throws a descriptive error if the file is missing or malformed so the
 * caller can prompt the user to run `gemini auth login`.
 */
export async function loadCredentials(
  path: string = resolveCredentialsPath(),
): Promise<GeminiOAuthCredentials> {
  let raw: string;
  try {
    raw = await fs.readFile(path, "utf8");
  } catch (cause) {
    throw new Error(
      `Gemini credentials not found at ${path}. Run 'gemini auth login' or set GEMINI_CREDENTIALS_PATH.`,
      { cause },
    );
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (cause) {
    throw new Error(`Credentials file at ${path} is not valid JSON.`, {
      cause,
    });
  }

  if (!isCredentials(parsed)) {
    throw new Error(
      `Credentials file at ${path} is missing required fields (access_token, refresh_token, expiry_date).`,
    );
  }

  return parsed;
}

/**
 * Return a valid (non-expired) access token, refreshing transparently if
 * the cached value has expired or is about to expire. On refresh, the new
 * token is persisted to disk so the canonical Gemini CLI also benefits.
 */
export async function getAccessToken(
  path: string = resolveCredentialsPath(),
): Promise<string> {
  const creds = await loadCredentials(path);
  const now = Date.now();
  if (creds.expiry_date - EXPIRY_GUARD_MS > now) {
    return creds.access_token;
  }
  const refreshed = await refresh(creds);
  await persistCredentials(path, refreshed);
  return refreshed.access_token;
}

/**
 * Build the Authorization header value (`Bearer <token>`) for downstream
 * HTTP requests to the Gemini / Code Assist APIs.
 */
export async function authorizationHeader(
  path: string = resolveCredentialsPath(),
): Promise<string> {
  return `Bearer ${await getAccessToken(path)}`;
}

// ── Internals ────────────────────────────────────────────────────────────────

function isCredentials(value: unknown): value is GeminiOAuthCredentials {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v["access_token"] === "string" &&
    typeof v["refresh_token"] === "string" &&
    typeof v["token_type"] === "string" &&
    typeof v["expiry_date"] === "number"
  );
}

/**
 * Exchange the refresh_token for a fresh access_token against the public
 * Google OAuth token endpoint.
 */
async function refresh(
  creds: GeminiOAuthCredentials,
): Promise<GeminiOAuthCredentials> {
  const body = new URLSearchParams({
    client_id: OAUTH_CLIENT_ID,
    client_secret: OAUTH_CLIENT_SECRET,
    refresh_token: creds.refresh_token,
    grant_type: "refresh_token",
  });

  const response = await fetch(GOOGLE_TOKEN_URL, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: body.toString(),
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Failed to refresh Gemini OAuth token (${response.status}): ${detail}`,
    );
  }

  const payload = (await response.json()) as GoogleRefreshResponse;
  return {
    ...creds,
    access_token: payload.access_token,
    token_type: payload.token_type,
    scope: payload.scope ?? creds.scope,
    id_token: payload.id_token ?? creds.id_token,
    expiry_date: Date.now() + payload.expires_in * 1000,
  };
}

/**
 * Write the refreshed credentials back to disk atomically (`tmp` + rename)
 * so a concurrent reader never sees a half-written file.
 */
async function persistCredentials(
  path: string,
  creds: GeminiOAuthCredentials,
): Promise<void> {
  const tmpPath = `${path}.${process.pid}.tmp`;
  const json = `${JSON.stringify(creds, null, 2)}\n`;
  await fs.writeFile(tmpPath, json, { mode: 0o600 });
  await fs.rename(tmpPath, path);
}
