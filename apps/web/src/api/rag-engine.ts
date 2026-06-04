// SPDX-License-Identifier: Apache-2.0
/**
 * rag-engine.ts — Production-ready RAG + LLM execution system for the Aphrody Web App.
 *
 * Implements:
 *   1. Redis Semantic Cache: caching queries & answers in local Redis (port 6379).
 *   2. Serverless Web Scraping: spawning `bxc scrape <url> --markdown` dynamically when URLs are in the prompt.
 *   3. Shenron RAG: SQLite-based hybrid search (BM25 + cosine vector search) + RRF fusion + Cross-Encoder reranking.
 *   4. Rpbey RAG: querying the public rpbey search API fallback.
 *   5. Gemini Live API: streaming SSE completions using the environment's `GEMINI_API_KEY`.
 */

import { Database } from "bun:sqlite";
import { redis } from "bun";
import { createHash } from "node:crypto";
import type { ChatMessage } from "./types.ts";

const SHENRON_DB_PATH = "/home/ubuntu/shenron/apps/bot/data/bot.db";
const EMBED_URL = process.env.EMBED_URL ?? "http://127.0.0.1:5007";
const RPBEY_SEARCH_URL = "https://rpbey.fr/api/v1/search";
const CACHE_SIMILARITY_THRESHOLD = 0.90;
const CACHE_TTL_SECONDS = 30 * 24 * 60 * 60; // 30 days
const RRF_K = 60;

export interface RagHit {
  rowid: number;
  kind: string;
  title: string;
  url: string;
  snippet: string;
  content?: string;
}

interface VectorCache {
  rowids: Int32Array;
  mat: Float32Array;
  dim: number;
  n: number;
  version: number;
}

let shenronVectorCache: VectorCache | null = null;

function md5(text: string): string {
  return createHash("md5").update(text.toLowerCase().trim()).digest("hex");
}

function cosineSimilarity(a: Float32Array | number[], b: Float32Array | number[]): number {
  let dot = 0;
  let normA = 0;
  let normB = 0;
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  if (normA === 0 || normB === 0) return 0;
  return dot / (Math.sqrt(normA) * Math.sqrt(normB));
}

/** Pre-loads the vector matrix from SQLite in a non-blocking way. */
function loadShenronVectors(): VectorCache | null {
  try {
    const db = new Database(SHENRON_DB_PATH, { readonly: true });
    const meta = db.query("SELECT built_at, dim FROM rag_meta LIMIT 1").get() as {
      built_at: number;
      dim: number;
    } | null;

    if (!meta) return null;
    const version = meta.built_at;
    const dim = meta.dim;

    if (shenronVectorCache && shenronVectorCache.version === version && shenronVectorCache.dim === dim) {
      return shenronVectorCache;
    }

    const rows = db.query("SELECT rowid, vec FROM rag_vectors ORDER BY rowid").all() as {
      rowid: number;
      vec: Uint8Array;
    }[];

    if (rows.length === 0) return null;

    const n = rows.length;
    const rowids = new Int32Array(n);
    const mat = new Float32Array(n * dim);

    for (let i = 0; i < n; i++) {
      rowids[i] = rows[i].rowid;
      const blob = rows[i].vec;
      const f32 = new Float32Array(blob.buffer, blob.byteOffset, blob.byteLength / 4);
      mat.set(f32.subarray(0, dim), i * dim);
    }

    shenronVectorCache = { rowids, mat, dim, n, version };
    db.close();
    return shenronVectorCache;
  } catch (err) {
    console.error("[RAG ENGINE] Failed to load Shenron vectors:", err);
    return null;
  }
}

/** Embeds a text using the local E5 sidecar. */
async function getEmbedding(text: string): Promise<Float32Array | null> {
  try {
    const res = await fetch(`${EMBED_URL}/embed`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ texts: [text], kind: "query" }),
      signal: AbortSignal.timeout(3000),
    });
    if (!res.ok) return null;
    const json = (await res.json()) as { vectors: number[][] };
    const vec = json.vectors?.[0];
    return vec ? Float32Array.from(vec) : null;
  } catch (err) {
    console.error("[RAG ENGINE] Failed to fetch embedding:", err);
    return null;
  }
}

/** Reranks passages with a cross-encoder model. */
async function rerankPassages(query: string, passages: string[]): Promise<number[] | null> {
  try {
    const res = await fetch(`${EMBED_URL}/rerank`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ query, passages }),
      signal: AbortSignal.timeout(5000),
    });
    if (!res.ok) return null;
    const json = (await res.json()) as { scores: number[] };
    return json.scores;
  } catch (err) {
    console.error("[RAG ENGINE] Reranking failed:", err);
    return null;
  }
}

/** Parses query into FTS5 terms. */
function ftsMatch(raw: string): string | null {
  const tokens = raw
    .replace(/["*()]/g, " ")
    .split(/\s+/)
    .filter((t) => t.length > 1);
  if (tokens.length === 0) return null;
  return tokens.map((t) => `"${t}"`).join(" OR ");
}

/** Computes exact cosine similarity top-K. */
function cosineTopK(c: VectorCache, q: Float32Array, k: number): { rowid: number; score: number }[] {
  const { mat, dim, n, rowids } = c;
  const scored: { rowid: number; score: number }[] = [];
  for (let i = 0; i < n; i++) {
    let dot = 0;
    const off = i * dim;
    for (let d = 0; d < dim; d++) {
      dot += mat[off + d] * q[d];
    }
    scored.push({ rowid: rowids[i], score: dot });
  }
  return scored.sort((a, b) => b.score - a.score).slice(0, k);
}

/** Executes native Shenron hybrid search. */
export async function searchShenron(query: string, limit = 5): Promise<RagHit[]> {
  try {
    const db = new Database(SHENRON_DB_PATH, { readonly: true });
    const match = ftsMatch(query);
    const POOL = 30;

    // 1. Lexical retrieval (FTS5)
    let lexicalHits: RagHit[] = [];
    if (match) {
      lexicalHits = db
        .query(
          "SELECT rowid, kind, title, url, snippet(rag_chunks, 3, '', '', '…', 18) AS snippet " +
            "FROM rag_chunks WHERE rag_chunks MATCH ? ORDER BY rank LIMIT ?",
        )
        .all(match, POOL) as RagHit[];
    }

    const lexicalMap = new Map<number, RagHit>();
    for (const h of lexicalHits) lexicalMap.set(h.rowid, h);

    // 2. Dense vector retrieval
    const cacheMat = loadShenronVectors();
    const qv = cacheMat ? await getEmbedding(query) : null;
    const denseHits = cacheMat && qv ? cosineTopK(cacheMat, qv, POOL) : [];

    if (denseHits.length === 0) {
      db.close();
      return lexicalHits.slice(0, limit);
    }

    // 3. RRF Fusion
    const fused = new Map<number, number>();
    lexicalHits.forEach((h, rank) => {
      fused.set(h.rowid, (fused.get(h.rowid) ?? 0) + 1 / (RRF_K + rank + 1));
    });
    denseHits.forEach((d, rank) => {
      fused.set(d.rowid, (fused.get(d.rowid) ?? 0) + 1 / (RRF_K + rank + 1));
    });

    const orderedIds = [...fused.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10)
      .map(([rowid]) => rowid);

    // 4. Hydrate content
    const ph = orderedIds.map(() => "?").join(",");
    const rows = db
      .query(`SELECT rowid, kind, title, url, content FROM rag_chunks WHERE rowid IN (${ph})`)
      .all(...orderedIds) as (RagHit & { content: string })[];

    const rowById = new Map<number, RagHit & { content: string }>();
    for (const r of rows) rowById.set(r.rowid, r);

    let candidates = orderedIds
      .map((id) => {
        const r = rowById.get(id);
        if (!r) return null;
        const snippet = lexicalMap.get(id)?.snippet ?? r.content.slice(0, 160);
        return {
          rowid: r.rowid,
          kind: r.kind,
          title: r.title,
          url: r.url,
          snippet,
          content: r.content,
        };
      })
      .filter((x): x is NonNullable<typeof x> => x !== null);

    // 5. Cross-Encoder Reranking
    if (candidates.length > 1) {
      const passages = candidates.map((c) => `${c.title}. ${c.content}`.slice(0, 400));
      const scores = await rerankPassages(query, passages);
      if (scores && scores.length === candidates.length) {
        candidates = candidates
          .map((cand, i) => ({ cand, score: scores[i] }))
          .sort((a, b) => b.score - a.score)
          .map((x) => x.cand);
      }
    }

    db.close();
    return candidates.slice(0, limit);
  } catch (err) {
    console.error("[RAG ENGINE] Shenron hybrid search failed:", err);
    return [];
  }
}

/** Queries public Rpbey API. */
export async function searchRpbey(query: string, limit = 5): Promise<RagHit[]> {
  try {
    const url = new URL(RPBEY_SEARCH_URL);
    url.searchParams.set("q", query);
    url.searchParams.set("limit", String(limit));

    const res = await fetch(url.toString(), {
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(4000),
    });

    if (!res.ok) return [];
    const json = (await res.json()) as { data?: any[] | { data?: any[] } };
    const arr = Array.isArray(json.data) ? json.data : (json.data?.data ?? []);

    return arr.map((item: any, i: number) => ({
      rowid: i + 1,
      kind: item.category || "wiki",
      title: item.title,
      url: item.url.startsWith("http") ? item.url : `https://rpbey.fr${item.url}`,
      snippet: item.details || item.subtitle || "",
      content: item.details || "",
    }));
  } catch (err) {
    console.error("[RAG ENGINE] Rpbey search failed:", err);
    return [];
  }
}

/** Dynamic serverless scraping of URLs using bxc CLI. */
export async function performServerlessScrape(url: string): Promise<string> {
  console.log(`[bxc serverless scrape] scraping URL: ${url}`);
  try {
    const proc = Bun.spawn(["/home/ubuntu/bxc/bin/bxc", "scrape", url, "--markdown"], {
      stdout: "pipe",
      stderr: "pipe",
    });

    const stdout = await new Response(proc.stdout).text();
    const code = await proc.exited;

    if (code !== 0) {
      const stderr = await new Response(proc.stderr).text();
      throw new Error(`bxc scrape process exited with ${code}. Error: ${stderr}`);
    }

    console.log(`[bxc serverless scrape] successfully scraped ${stdout.length} characters.`);
    return stdout;
  } catch (err) {
    console.error(`[bxc serverless scrape] failed for ${url}:`, err);
    return `[Failed to scrape ${url}]`;
  }
}

/** Tries to lookup query in Redis semantic cache. */
export async function getSemanticCache(query: string, model: string): Promise<string | null> {
  try {
    const qv = await getEmbedding(query);
    if (!qv) return null;

    const hash = md5(query);
    const lruKey = `dbz:scache:${model}:lru`;

    const hashes = (await redis.zrange(lruKey, -100, -1)) as string[];
    if (!hashes || hashes.length === 0) return null;

    const promises: Promise<string | null>[] = [];
    for (const h of hashes) {
      promises.push(redis.get(`dbz:scache:${model}:${h}:vector`));
    }
    const vectorsJson = await Promise.all(promises);

    let bestSimilarity = -1;
    let bestHash: string | null = null;

    for (let i = 0; i < hashes.length; i++) {
      const vecJson = vectorsJson[i];
      if (!vecJson) continue;
      try {
        const cachedVector = JSON.parse(vecJson) as number[];
        const sim = cosineSimilarity(qv, cachedVector);
        if (sim > bestSimilarity) {
          bestSimilarity = sim;
          bestHash = hashes[i];
        }
      } catch {
        // corrupt cache items
      }
    }

    if (bestHash && bestSimilarity >= CACHE_SIMILARITY_THRESHOLD) {
      const answer = await redis.get(`dbz:scache:${model}:${bestHash}:answer`);
      if (answer) {
        await redis.zadd(lruKey, Date.now(), bestHash); // refresh score
        console.log(`[SEMANTIC CACHE HIT] Model: ${model}, Similarity: ${bestSimilarity.toFixed(4)}`);
        return answer;
      }
    }
    return null;
  } catch (err) {
    console.error("[SEMANTIC CACHE] Failed to get cache:", err);
    return null;
  }
}

/** Stores query + answer + embedding vector in Redis semantic cache. */
export async function setSemanticCache(query: string, answer: string, model: string): Promise<void> {
  if (!answer || answer.trim().length === 0) return;
  try {
    const qv = await getEmbedding(query);
    if (!qv) return;

    const hash = md5(query);
    const lruKey = `dbz:scache:${model}:lru`;

    await Promise.all([
      redis.set(`dbz:scache:${model}:${hash}:query`, query, "EX", CACHE_TTL_SECONDS),
      redis.set(`dbz:scache:${model}:${hash}:answer`, answer, "EX", CACHE_TTL_SECONDS),
      redis.set(`dbz:scache:${model}:${hash}:vector`, JSON.stringify(Array.from(qv)), "EX", CACHE_TTL_SECONDS),
      redis.zadd(lruKey, Date.now(), hash),
    ]);
  } catch (err) {
    console.error("[SEMANTIC CACHE] Failed to write cache:", err);
  }
}

/** Custom system instruction based on model. */
export function getSystemInstruction(model: string, contextMd: string): string {
  let persona = "You are a helpful assistant.";
  if (model === "shenron") {
    persona = `Tu es Shenron, le Dragon Sacré de la Terre. Réponds de manière solennelle, majestueuse, brève et grave, comme un être divin accordant un vœu (ex. 'Je t'écoute', 'Ton vœu est exaucé dans la limite de mes pouvoirs').`;
  } else if (model === "rpbey") {
    persona = `Tu es Ryuga, le légendaire blader de L-Drago. Réponds avec arrogance, colère et fierté, tout en transmettant des informations précises sur le jeu Beyblade. Tu as un ton provocant et tu aimes les bladers passionnés.`;
  }

  return `${persona}
Appuie-toi UNIQUEMENT sur les faits décrits dans le contexte suivant pour rédiger ta réponse. Si le contexte ne contient pas l'information requise pour répondre, réponds poliment que tu ne trouves pas cela dans les archives.

[Contexte RAG]
${contextMd}`;
}

/** Call Gemini API to stream response to the client. */
export async function streamGeminiChat(
  systemInstruction: string,
  messages: { role: string; content: string }[],
  onChunk: (text: string) => void,
): Promise<string> {
  const apiKey = process.env.GEMINI_API_KEY || process.env.GOOGLE_API_KEY;
  if (!apiKey) {
    throw new Error("GEMINI_API_KEY or GOOGLE_API_KEY is not defined in the environment.");
  }

  // Map role shapes
  const geminiContents = messages.map((m) => ({
    role: m.role === "assistant" ? "model" : "user",
    parts: [{ text: m.content }],
  }));

  const url = `https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?key=${apiKey}`;

  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      contents: geminiContents,
      systemInstruction: {
        parts: [{ text: systemInstruction }],
      },
      generationConfig: {
        temperature: 0.25,
        maxOutputTokens: 2048,
      },
    }),
  });

  if (!res.ok) {
    const errText = await res.text();
    throw new Error(`Gemini API error ${res.status}: ${errText}`);
  }

  const reader = res.body?.getReader();
  if (!reader) throw new Error("Could not get stream reader");

  const decoder = new TextDecoder();
  let buffer = "";
  let fullAnswer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    
    // Parse JSON stream chunks
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";

    for (const line of lines) {
      const cleanLine = line.trim();
      if (!cleanLine) continue;

      try {
        let jsonStr = cleanLine;
        if (cleanLine.startsWith("data:")) {
          jsonStr = cleanLine.substring(5).trim();
        }
        
        // Remove trailing commas if wrapped in array
        if (jsonStr.startsWith(",")) jsonStr = jsonStr.substring(1).trim();
        if (jsonStr.startsWith("[") || jsonStr.endsWith("]")) continue;

        const data = JSON.parse(jsonStr);
        const chunkText = data.candidates?.[0]?.content?.parts?.[0]?.text;
        if (chunkText) {
          fullAnswer += chunkText;
          onChunk(chunkText);
        }
      } catch {
        // Line might be incomplete JSON, keep in buffer
        buffer = cleanLine + "\n" + buffer;
      }
    }
  }

  return fullAnswer;
}
