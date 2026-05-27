// SPDX-License-Identifier: Apache-2.0
//! HTTP Server & CLI Entrypoint for the design compiler engine.

import {DesignCompiler} from './generator.ts';

// Parse command line arguments
const args = Bun.argv;
const promptIndex = args.indexOf('--prompt');
const colorIndex = args.indexOf('--color');
const designSystemIndex = args.indexOf('--design-system');

const compiler = new DesignCompiler();

if (promptIndex !== -1) {
  // Run as CLI tool
  const prompt = args[promptIndex + 1];
  const color = colorIndex !== -1 ? args[colorIndex + 1] : undefined;
  const designSystem =
    designSystemIndex !== -1 ? args[designSystemIndex + 1] : undefined;

  if (!prompt) {
    console.error('Error: Please provide a prompt value using --prompt <text>');
    process.exit(1);
  }

  console.log(`[CLI Compiler] Compiling prompt: "${prompt}"...`);
  if (designSystem) {
    console.log(`[CLI Compiler] Using custom design system: "${designSystem}"`);
  }
  const result = compiler.compile(prompt, color, designSystem);

  console.log('\n--- DESIGN BRIEF EXTRACTED ---');
  console.log(JSON.stringify(result.brief, null, 2));

  console.log('\n--- HCT SEED COLOR GENERATED ---');
  console.log(`Seed Color: ${result.theme.seed}`);
  console.log(`Primary (Tone 40): ${result.theme.palettes.primary.tones[40]}`);
  console.log(`Neutral (Tone 98): ${result.theme.palettes.neutral.tones[98]}`);

  console.log('\n--- GENERATED FILES ---');
  for (const file of result.files) {
    console.log(`- ${file.path} (${file.content.length} characters)`);
  }

  console.log('\n--- SELF-CRITIQUE ---');
  console.log(`Scores: ${JSON.stringify(result.critique.scores)}`);
  console.log(`Rationale: ${result.critique.rationale}`);

  process.exit(0);
} else {
  // Start Bun HTTP server
  const PORT = 3005;
  console.log(
    `[HTTP Server] Starting automated design engine on http://localhost:${PORT}...`,
  );

  Bun.serve({
    port: PORT,
    async fetch(req) {
      const url = new URL(req.url);

      if (req.method === 'POST' && url.pathname === '/api/generate') {
        try {
          const body = await req.json();
          const {prompt, seedColor, designSystemId} = body;

          if (!prompt) {
            return Response.json(
              {error: 'Missing prompt field'},
              {status: 400},
            );
          }

          const result = compiler.compile(prompt, seedColor, designSystemId);
          return Response.json(result);
        } catch (e: unknown) {
          const message = e instanceof Error ? e.message : String(e);
          return Response.json({error: message}, {status: 500});
        }
      }

      if (req.method === 'POST' && url.pathname === '/api/generate/finalize') {
        try {
          const body = await req.json();
          const {
            projectId,
            apiKey,
            model,
            protocol = 'anthropic',
            baseUrl,
            transcriptJsonl = '',
            transcriptMessageCount = 0,
            designSystemId,
            designSystemBody,
            artifact,
          } = body;

          if (!projectId) {
            return Response.json(
              {error: 'Missing projectId field'},
              {status: 400},
            );
          }
          if (!apiKey) {
            return Response.json(
              {error: 'Missing apiKey field'},
              {status: 400},
            );
          }
          if (!model) {
            return Response.json({error: 'Missing model field'}, {status: 400});
          }

          const defaultBaseUrl =
            protocol === 'openai'
              ? 'https://api.openai.com'
              : protocol === 'google'
                ? 'https://generativelanguage.googleapis.com'
                : 'https://api.anthropic.com';

          const resolvedBaseUrl = (baseUrl || defaultBaseUrl).replace(
            /\/+$/,
            '',
          );

          const prompts = buildSynthesisPrompt({
            projectId,
            transcriptJsonl,
            transcriptMessageCount,
            designSystemId,
            designSystemBody,
            artifact,
            now: new Date(),
          });

          let response: Response;
          if (protocol === 'anthropic') {
            const endpoint = resolvedBaseUrl.includes('/v1')
              ? `${resolvedBaseUrl}/messages`
              : `${resolvedBaseUrl}/v1/messages`;
            response = await fetch(endpoint, {
              method: 'POST',
              headers: {
                'content-type': 'application/json',
                'x-api-key': apiKey,
                'anthropic-version': '2023-06-01',
              },
              body: JSON.stringify({
                model,
                max_tokens: 4000,
                system: prompts.systemPrompt,
                messages: [{role: 'user', content: prompts.userPrompt}],
              }),
            });
          } else if (protocol === 'openai') {
            const endpoint = resolvedBaseUrl.includes('/v1')
              ? `${resolvedBaseUrl}/chat/completions`
              : `${resolvedBaseUrl}/v1/chat/completions`;
            response = await fetch(endpoint, {
              method: 'POST',
              headers: {
                'content-type': 'application/json',
                authorization: `Bearer ${apiKey}`,
              },
              body: JSON.stringify({
                model,
                messages: [
                  {role: 'system', content: prompts.systemPrompt},
                  {role: 'user', content: prompts.userPrompt},
                ],
                max_tokens: 4000,
              }),
            });
          } else if (protocol === 'google') {
            const endpoint = `${resolvedBaseUrl}/v1beta/models/${model}:generateContent?key=${apiKey}`;
            response = await fetch(endpoint, {
              method: 'POST',
              headers: {
                'content-type': 'application/json',
              },
              body: JSON.stringify({
                contents: [
                  {
                    role: 'user',
                    parts: [{text: prompts.userPrompt}],
                  },
                ],
                systemInstruction: {
                  parts: [{text: prompts.systemPrompt}],
                },
                generationConfig: {
                  maxOutputTokens: 4000,
                },
              }),
            });
          } else {
            return Response.json(
              {error: `Unsupported protocol: ${protocol}`},
              {status: 400},
            );
          }

          if (!response.ok) {
            const text = await response.text().catch(() => '');
            return Response.json(
              {
                error: `Upstream ${protocol} call failed with status ${response.status}`,
                details: text,
              },
              {status: 502},
            );
          }

          const payload = await response.json();
          const designMd = extractDesignMd(payload, protocol);

          return Response.json({
            success: true,
            designMd,
            projectId,
            designSystemId: designSystemId || null,
          });
        } catch (e: unknown) {
          const message = e instanceof Error ? e.message : String(e);
          return Response.json({error: message}, {status: 500});
        }
      }

      // Root details
      return Response.json({
        engine: 'design',
        status: 'active',
        endpoints: {
          'POST /api/generate': {
            body: {
              prompt: 'string (required)',
              seedColor: 'string (optional HEX seed)',
              designSystemId: 'string (optional design system override)',
            },
          },
          'POST /api/generate/finalize': {
            body: {
              projectId: 'string (required)',
              apiKey: 'string (required)',
              model: 'string (required)',
              protocol: 'string (optional: anthropic | openai | google)',
              baseUrl: 'string (optional API base url)',
              transcriptJsonl: 'string (optional)',
              transcriptMessageCount: 'number (optional)',
              designSystemId: 'string (optional)',
              designSystemBody: 'string (optional)',
              artifact: 'object with name and body (optional)',
            },
          },
        },
      });
    },
  });
}

// Synthesis Prompts compiler helper
function buildSynthesisPrompt(input: {
  projectId: string;
  transcriptJsonl: string;
  transcriptMessageCount: number;
  designSystemId?: string | null;
  designSystemBody?: string | null;
  artifact?: {name: string; body: string} | null;
  now: Date;
}) {
  const SYSTEM_PROMPT = `You are a senior product designer synthesizing a finalized design package
from a multi-turn design session. Your output is a single Markdown document
named DESIGN.md that captures the durable design intent of the work so a
fresh contributor (human or LLM) can reconstruct context without replaying
the full chat.

Output structure (Markdown headings exactly as below):
# DESIGN.md
## Summary
## Brand & Voice
## Information Architecture
## Components & Patterns
## Visual System
## Open Questions
## Provenance

The Provenance section MUST list:
- Project ID
- Design system (or "none" if not selected)
- Current artifact (file name, or "none" if not in scope)
- Transcript message count
- Generated UTC timestamp

Render Provenance fields as plain Markdown bullets with no emphasis on the field labels, exactly: "- Field name: value". Do not bold, italicize, or otherwise decorate the labels or the colon. Field values may use inline code formatting (backticks) where appropriate.

Output the Markdown body only. No preamble, no chat-style framing, no
"Here's your DESIGN.md" prefix. Do not invent facts not supported by the
inputs; if an input is missing or empty, the corresponding section should
say so explicitly rather than fabricating content.`;

  const designSystemHeader = input.designSystemId ?? 'none';
  const designSystemBody =
    input.designSystemBody && input.designSystemBody.trim().length > 0
      ? input.designSystemBody
      : '(no design system selected for this project)';

  const artifactHeader = input.artifact ? input.artifact.name : 'none';
  const artifactBody = input.artifact
    ? input.artifact.body
    : '(no artifact in scope for this finalize)';

  const userPrompt =
    `The following inputs describe the design session for project ${input.projectId}.\n\n` +
    `## Transcript (JSONL)\n${input.transcriptJsonl}\n\n` +
    `## Active design system: ${designSystemHeader}\n${designSystemBody}\n\n` +
    `## Current artifact: ${artifactHeader}\n${artifactBody}\n\n` +
    `## Generation context\n` +
    `- Generated at: ${input.now.toISOString()}\n` +
    `- Project ID: ${input.projectId}\n` +
    `- Transcript message count: ${input.transcriptMessageCount}\n\n` +
    `Synthesize DESIGN.md per the system instructions.`;

  return {systemPrompt: SYSTEM_PROMPT, userPrompt};
}

// Extractor helper for provider payloads
function extractDesignMd(payload: unknown, protocol: string): string {
  if (!payload || typeof payload !== 'object') {
    throw new Error('response payload was not an object');
  }

  const p = payload as Record<string, unknown>;
  let out = '';

  if (protocol === 'anthropic') {
    const content = p.content;
    if (Array.isArray(content)) {
      for (const block of content) {
        if (block && typeof block === 'object') {
          const blockObj = block as Record<string, unknown>;
          if (blockObj.type === 'text' && typeof blockObj.text === 'string') {
            out += blockObj.text;
          }
        }
      }
    }
  } else if (protocol === 'openai') {
    const choices = p.choices;
    if (Array.isArray(choices)) {
      for (const choice of choices) {
        if (choice && typeof choice === 'object') {
          const choiceObj = choice as Record<string, unknown>;
          const message = choiceObj.message;
          if (message && typeof message === 'object') {
            const msgObj = message as Record<string, unknown>;
            if (typeof msgObj.content === 'string') {
              out += msgObj.content;
            }
          }
        }
      }
    }
  } else if (protocol === 'google') {
    const candidates = p.candidates;
    if (Array.isArray(candidates)) {
      for (const candidate of candidates) {
        if (candidate && typeof candidate === 'object') {
          const candObj = candidate as Record<string, unknown>;
          const content = candObj.content;
          if (content && typeof content === 'object') {
            const contentObj = content as Record<string, unknown>;
            const parts = contentObj.parts;
            if (Array.isArray(parts)) {
              for (const part of parts) {
                if (part && typeof part === 'object') {
                  const partObj = part as Record<string, unknown>;
                  if (typeof partObj.text === 'string') {
                    out += partObj.text;
                  }
                }
              }
            }
          }
        }
      }
    }
  }

  if (out.length === 0) {
    throw new Error(`upstream ${protocol} response contained no text content`);
  }
  return out;
}
