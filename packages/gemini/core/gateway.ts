// SPDX-License-Identifier: Apache-2.0
//
// OpenAI-compatible chat completions gateway for the whisper-gateway
// backend.
//
// The whisper-gateway path deliberately avoids the Gemini API entirely.
// Instead, we POST to any endpoint that speaks the OpenAI chat
// completions wire format — that includes:
//
//   - Cloudflare AI Gateway (https://gateway.ai.cloudflare.com/v1/...)
//   - Vercel AI Gateway (https://gateway.ai.vercel.app/v1/...)
//   - OpenAI itself (https://api.openai.com)
//   - Local llama.cpp server, vLLM, or LM Studio
//
// Mirrors the adapters under crates/aphrody-gateway/ (cloudflare.rs,
// vercel.rs, openai_proxy.rs) for consistency with the Rust side.

// ── Public types ─────────────────────────────────────────────────────────────

/** Role on a single chat message. */
export type ChatRole = "system" | "user" | "assistant" | "tool";

/** A single chat message. */
export interface ChatMessage {
  role: ChatRole;
  content: string;
  /** Optional name for `tool` / `assistant` messages. */
  name?: string;
}

/** Options accepted by [`chatCompletion`]. */
export interface ChatRequest {
  messages: ReadonlyArray<ChatMessage>;
  /** Optional model id. Falls back to `APHRODY_GATEWAY_MODEL`. */
  model?: string;
  /** Sampling temperature (0.0 – 2.0). */
  temperature?: number;
  /** Top-p nucleus sampling. */
  top_p?: number;
  /** Max completion tokens. */
  max_tokens?: number;
  /** Optional stop sequences. */
  stop?: ReadonlyArray<string>;
}

/** Normalized assistant reply. */
export interface ChatResponse {
  /** The full assistant message text. */
  text: string;
  /** Model id reported by the gateway. */
  model: string;
  /** Token usage if the gateway reports it. */
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

/** Gateway configuration. All fields default to env vars. */
export interface GatewayConfig {
  /** Full URL to POST to (defaults to `APHRODY_GATEWAY_URL`). */
  url?: string;
  /** Bearer token (defaults to `APHRODY_GATEWAY_TOKEN`). */
  token?: string;
  /** Model id (defaults to `APHRODY_GATEWAY_MODEL`). */
  model?: string;
}

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * POST a chat completion request to the configured OpenAI-compatible
 * gateway. Returns the assistant reply as a normalized
 * [`ChatResponse`].
 *
 * Throws if `APHRODY_GATEWAY_URL` is not set and `config.url` is also
 * absent — there is no implicit fallback to api.openai.com in order to
 * prevent accidental egress to a vendor the user did not opt in to.
 */
export async function chatCompletion(
  request: ChatRequest,
  config: GatewayConfig = {},
): Promise<ChatResponse> {
  const url = config.url ?? process.env["APHRODY_GATEWAY_URL"];
  if (!url || url.length === 0) {
    throw new Error(
      "APHRODY_GATEWAY_URL is unset. Point it at an OpenAI-compatible chat completions endpoint.",
    );
  }
  const token = config.token ?? process.env["APHRODY_GATEWAY_TOKEN"];
  const model =
    request.model ??
    config.model ??
    process.env["APHRODY_GATEWAY_MODEL"] ??
    "gpt-4o-mini";

  const body = {
    model,
    messages: request.messages,
    ...(request.temperature !== undefined && { temperature: request.temperature }),
    ...(request.top_p !== undefined && { top_p: request.top_p }),
    ...(request.max_tokens !== undefined && { max_tokens: request.max_tokens }),
    ...(request.stop !== undefined && { stop: [...request.stop] }),
  };

  const headers: Record<string, string> = {
    "content-type": "application/json",
  };
  if (token) {
    headers["authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(joinChatCompletionsPath(url), {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Gateway chat completion failed (${response.status}): ${detail}`,
    );
  }

  const payload = (await response.json()) as Record<string, unknown>;
  return normalizeResponse(payload, model);
}

// ── Internals ────────────────────────────────────────────────────────────────

/**
 * If the user passed a base URL (e.g. `https://gateway.openai.com/v1`),
 * append `/chat/completions`. If they passed the full URL already, leave
 * it alone.
 */
function joinChatCompletionsPath(url: string): string {
  if (url.endsWith("/chat/completions")) return url;
  const trimmed = url.replace(/\/+$/u, "");
  return `${trimmed}/chat/completions`;
}

function normalizeResponse(
  payload: Record<string, unknown>,
  fallbackModel: string,
): ChatResponse {
  const choices = payload["choices"];
  let text = "";
  if (Array.isArray(choices) && choices.length > 0) {
    const first = choices[0] as Record<string, unknown>;
    const message = first["message"] as Record<string, unknown> | undefined;
    if (message && typeof message["content"] === "string") {
      text = message["content"];
    }
  }

  const model =
    typeof payload["model"] === "string" ? payload["model"] : fallbackModel;

  const usageRaw = payload["usage"] as Record<string, unknown> | undefined;
  const usage = usageRaw
    ? {
        prompt_tokens:
          typeof usageRaw["prompt_tokens"] === "number"
            ? usageRaw["prompt_tokens"]
            : 0,
        completion_tokens:
          typeof usageRaw["completion_tokens"] === "number"
            ? usageRaw["completion_tokens"]
            : 0,
        total_tokens:
          typeof usageRaw["total_tokens"] === "number"
            ? usageRaw["total_tokens"]
            : 0,
      }
    : undefined;

  return { text, model, usage };
}
