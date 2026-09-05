<!-- SPDX-License-Identifier: Apache-2.0 -->

# Openclaw extensions — port-priority roadmap

**Source:** `C:/worktree/openclaw/extensions`
**Generated:** 2026-05-17T21:18:26.747Z
**Total extensions:** 122
**Distribution:** high=6, medium=20, low=96

Heuristic: `high` = name/keyword matches /memory|gateway|voice|sdk/; `medium` = known channel adapter; `low` = everything else.

## Ranked table

| priority | name | LOC | tests | deps | description |
|---|---|---:|---:|---:|---|
| high | `@openclaw/memory-core` | 48066 | 56 | 3 | OpenClaw core memory search plugin |
| high | `@openclaw/voice-call` | 28110 | 45 | 4 | OpenClaw voice-call plugin |
| high | `@openclaw/memory-wiki` | 15945 | 23 | 3 | OpenClaw persistent wiki plugin |
| high | `@openclaw/memory-lancedb` | 4228 | 3 | 4 | OpenClaw LanceDB-backed long-term memory plugin with auto-recall/capture |
| high | `@openclaw/cloudflare-ai-gateway-provider` | 709 | 3 | 0 | OpenClaw Cloudflare AI Gateway provider plugin |
| high | `@openclaw/vercel-ai-gateway-provider` | 606 | 2 | 0 | OpenClaw Vercel AI Gateway provider plugin |
| medium | `@openclaw/discord` | 101608 | 148 | 7 | OpenClaw Discord channel plugin |
| medium | `@openclaw/telegram` | 87076 | 119 | 5 | OpenClaw Telegram channel plugin |
| medium | `@openclaw/matrix` | 80278 | 118 | 8 | OpenClaw Matrix channel plugin |
| medium | `@openclaw/slack` | 55052 | 95 | 6 | OpenClaw Slack channel plugin |
| medium | `@openclaw/feishu` | 49865 | 62 | 3 | OpenClaw Feishu/Lark channel plugin (community maintained by @m1heng) |
| medium | `@openclaw/whatsapp` | 37379 | 74 | 5 | OpenClaw WhatsApp channel plugin |
| medium | `@openclaw/qqbot` | 30239 | 50 | 5 | OpenClaw QQ Bot channel plugin |
| medium | `@openclaw/mattermost` | 21705 | 40 | 2 | OpenClaw Mattermost channel plugin |
| medium | `@openclaw/imessage` | 18086 | 41 | 0 | OpenClaw iMessage channel plugin using imsg on a signed-in Mac |
| medium | `@openclaw/line` | 15784 | 24 | 2 | OpenClaw LINE channel plugin |
| medium | `@openclaw/signal` | 13133 | 23 | 1 | OpenClaw Signal channel plugin |
| medium | `@openclaw/zalouser` | 11353 | 20 | 3 | OpenClaw Zalo Personal Account plugin via native zca-js integration |
| medium | `@openclaw/tlon` | 9558 | 15 | 5 | OpenClaw Tlon/Urbit channel plugin |
| medium | `@openclaw/nostr` | 8786 | 12 | 2 | OpenClaw Nostr channel plugin for NIP-04 encrypted DMs |
| medium | `@openclaw/zalo` | 8052 | 24 | 1 | OpenClaw Zalo channel plugin |
| medium | `@openclaw/twitch` | 7162 | 15 | 4 | OpenClaw Twitch channel plugin |
| medium | `@openclaw/nextcloud-talk` | 6021 | 14 | 1 | OpenClaw Nextcloud Talk channel plugin |
| medium | `@openclaw/synology-chat` | 5292 | 7 | 1 | Synology Chat channel plugin for OpenClaw |
| medium | `@openclaw/irc` | 5157 | 16 | 1 | OpenClaw IRC channel plugin |
| medium | `@openclaw/qa-channel` | 2568 | 4 | 2 | OpenClaw QA synthetic channel plugin |
| low | `@openclaw/qa-lab` | 65433 | 75 | 5 | OpenClaw QA lab plugin with private debugger UI and scenario runner |
| low | `@openclaw/codex` | 64594 | 54 | 5 | OpenClaw Codex harness and model provider plugin |
| low | `@openclaw/browser-plugin` | 58926 | 104 | 6 | OpenClaw browser tool plugin |
| low | `@openclaw/msteams` | 38279 | 66 | 7 | OpenClaw Microsoft Teams channel plugin |
| low | `@openclaw/qa-matrix` | 28235 | 17 | 1 | OpenClaw Matrix QA runner plugin |
| low | `@openclaw/google-meet` | 20049 | 10 | 2 | OpenClaw Google Meet participant plugin |
| low | `@openclaw/google-plugin` | 17239 | 24 | 2 | OpenClaw Google plugin |
| low | `@openclaw/openai-provider` | 16131 | 27 | 2 | OpenClaw OpenAI provider plugins |
| low | `@openclaw/oc-path` | 12106 | 40 | 4 | OpenClaw oc:// workspace path plugin |
| low | `@openclaw/ollama-provider` | 11006 | 16 | 2 | OpenClaw Ollama provider plugin |
| low | `@openclaw/googlechat` | 9506 | 16 | 3 | OpenClaw Google Chat channel plugin |
| low | `@openclaw/xai-plugin` | 9419 | 23 | 2 | OpenClaw xAI plugin |
| low | `@openclaw/acpx` | 9003 | 12 | 4 | OpenClaw ACP runtime backend |
| low | `@openclaw/diffs` | 8754 | 9 | 5 | OpenClaw diff viewer plugin |
| low | `@openclaw/file-transfer` | 6550 | 12 | 2 | OpenClaw file transfer plugin (file_fetch, dir_list, dir_fetch, file_write) |
| low | `@openclaw/lmstudio-provider` | 6141 | 6 | 1 | OpenClaw LM Studio provider plugin |
| low | `@openclaw/canvas-plugin` | 5192 | 13 | 6 | OpenClaw Canvas plugin |
| low | `@openclaw/diagnostics-otel` | 5165 | 1 | 11 | OpenClaw diagnostics OpenTelemetry exporter |
| low | `@openclaw/minimax-provider` | 5130 | 11 | 0 | OpenClaw MiniMax provider and OAuth plugin |
| low | `@openclaw/openrouter-provider` | 4896 | 9 | 0 | OpenClaw OpenRouter provider plugin |
| low | `@openclaw/amazon-bedrock-provider` | 4302 | 7 | 5 | OpenClaw Amazon Bedrock provider plugin |
| low | `@openclaw/github-copilot-provider` | 4263 | 10 | 1 | OpenClaw GitHub Copilot provider plugin |
| low | `@openclaw/anthropic-provider` | 3654 | 6 | 1 | OpenClaw Anthropic provider plugin |
| low | `@openclaw/speech-core` | 3152 | 2 | 0 | OpenClaw speech runtime package |
| low | `@openclaw/fal-provider` | 2929 | 4 | 0 | OpenClaw fal provider plugin |
| low | `@openclaw/openshell-sandbox` | 2704 | 5 | 2 | OpenClaw OpenShell sandbox backend |
| low | `@openclaw/microsoft-foundry` | 2555 | 1 | 0 | OpenClaw Microsoft Foundry provider plugin |
| low | `@openclaw/migrate-hermes` | 2520 | 7 | 1 | Hermes to OpenClaw migration provider |
| low | `@openclaw/lobster` | 2456 | 3 | 3 | Lobster workflow tool plugin (typed pipelines + resumable approvals) |
| low | `@openclaw/skill-workshop` | 2441 | 1 | 1 | OpenClaw skill workshop plugin |
| low | `@openclaw/firecrawl-plugin` | 2265 | 1 | 1 | OpenClaw Firecrawl plugin |
| low | `@openclaw/bonjour` | 2255 | 5 | 1 | OpenClaw Bonjour/mDNS gateway discovery |
| low | `@openclaw/comfy-provider` | 2188 | 6 | 0 | OpenClaw ComfyUI provider plugin |
| low | `@openclaw/deepinfra-provider` | 2124 | 10 | 0 | OpenClaw DeepInfra provider plugin |
| low | `@openclaw/elevenlabs-speech` | 2090 | 6 | 0 | OpenClaw ElevenLabs speech plugin |
| low | `@openclaw/brave-plugin` | 1838 | 1 | 0 | OpenClaw Brave plugin |
| low | `@openclaw/clickclack` | 1641 | 4 | 2 | OpenClaw ClickClack channel plugin |
| low | `@openclaw/chutes-provider` | 1568 | 2 | 0 | OpenClaw Chutes.ai provider plugin |
| low | `@openclaw/webhooks` | 1514 | 3 | 1 | OpenClaw webhook bridge plugin |
| low | `@openclaw/moonshot-provider` | 1513 | 5 | 0 | OpenClaw Moonshot provider plugin |
| low | `@openclaw/amazon-bedrock-mantle-provider` | 1498 | 3 | 3 | OpenClaw Amazon Bedrock Mantle (OpenAI-compatible) provider plugin |
| low | `@openclaw/kimi-provider` | 1449 | 6 | 1 | OpenClaw Kimi provider plugin |
| low | `@openclaw/zai-provider` | 1398 | 5 | 0 | OpenClaw Z.AI provider plugin |
| low | `@openclaw/anthropic-vertex-provider` | 1339 | 6 | 3 | OpenClaw Anthropic Vertex provider plugin |
| low | `@openclaw/qwen-provider` | 1319 | 7 | 0 | OpenClaw Qwen Cloud provider plugin |
| low | `@openclaw/xiaomi-provider` | 1301 | 4 | 0 | OpenClaw Xiaomi provider plugin |
| low | `@openclaw/vydra-provider` | 1298 | 6 | 0 | OpenClaw Vydra media provider plugin |
| low | `@openclaw/migrate-claude` | 1279 | 1 | 0 | Claude to OpenClaw migration provider |
| low | `@openclaw/deepseek-provider` | 1234 | 3 | 0 | OpenClaw DeepSeek provider plugin |
| low | `@openclaw/mistral-provider` | 1154 | 6 | 0 | OpenClaw Mistral provider plugin |
| low | `@openclaw/tavily-plugin` | 1132 | 2 | 1 | OpenClaw Tavily plugin |
| low | `@openclaw/volcengine-provider` | 1115 | 3 | 0 | OpenClaw Volcengine provider plugin |
| low | `@openclaw/diagnostics-prometheus` | 1109 | 1 | 0 | OpenClaw diagnostics Prometheus exporter |
| low | `@openclaw/inworld-speech` | 1074 | 3 | 0 | OpenClaw Inworld speech plugin |
| low | `@openclaw/azure-speech` | 987 | 3 | 0 | OpenClaw Azure Speech plugin |
| low | `@openclaw/perplexity-plugin` | 981 | 1 | 0 | OpenClaw Perplexity plugin |
| low | `@openclaw/exa-plugin` | 971 | 1 | 0 | OpenClaw Exa plugin |
| low | `@openclaw/kilocode-provider` | 956 | 4 | 0 | OpenClaw Kilo Gateway provider plugin |
| low | `@openclaw/microsoft-speech` | 949 | 3 | 1 | OpenClaw Microsoft speech plugin |
| low | `@openclaw/searxng-plugin` | 941 | 2 | 0 | OpenClaw SearXNG plugin |
| low | `@openclaw/byteplus-provider` | 915 | 4 | 0 | OpenClaw BytePlus provider plugin |
| low | `@openclaw/venice-provider` | 858 | 3 | 0 | OpenClaw Venice provider plugin |
| low | `@openclaw/litellm-provider` | 799 | 3 | 0 | OpenClaw LiteLLM provider plugin |
| low | `@openclaw/tts-local-cli` | 773 | 1 | 0 | OpenClaw local CLI TTS plugin |
| low | `@openclaw/runway-provider` | 728 | 2 | 0 | OpenClaw Runway video provider plugin |
| low | `@openclaw/deepgram-provider` | 706 | 3 | 0 | OpenClaw Deepgram media-understanding provider |
| low | `@openclaw/opencode-go-provider` | 699 | 4 | 0 | OpenClaw OpenCode Go provider plugin |
| low | `@openclaw/llm-task` | 658 | 1 | 1 | OpenClaw JSON-only LLM task plugin |
| low | `@openclaw/fireworks-provider` | 599 | 2 | 1 | OpenClaw Fireworks provider plugin |
| low | `@openclaw/vllm-provider` | 591 | 2 | 0 | OpenClaw vLLM provider plugin |
| low | `@openclaw/duckduckgo-plugin` | 579 | 1 | 0 | OpenClaw DuckDuckGo plugin |
| low | `@openclaw/together-provider` | 527 | 2 | 0 | OpenClaw Together provider plugin |
| low | `@openclaw/gradium-speech` | 526 | 4 | 0 | OpenClaw Gradium speech plugin |
| low | `@openclaw/huggingface-provider` | 520 | 2 | 0 | OpenClaw Hugging Face provider plugin |
| low | `@openclaw/arcee-provider` | 510 | 1 | 0 | OpenClaw Arcee provider plugin |
| low | `@openclaw/admin-http-rpc` | 509 | 2 | 0 | OpenClaw admin HTTP RPC endpoint |
| low | `@openclaw/voyage-provider` | 472 | 0 | 0 | OpenClaw Voyage embedding provider plugin |
| low | `@openclaw/nvidia-provider` | 379 | 4 | 0 | OpenClaw NVIDIA provider plugin |
| low | `@openclaw/stepfun-provider` | 365 | 0 | 0 | OpenClaw StepFun provider plugin |
| low | `@openclaw/opencode-provider` | 364 | 5 | 0 | OpenClaw OpenCode Zen provider plugin |
| low | `@openclaw/document-extract-plugin` | 362 | 1 | 1 | OpenClaw local document extraction plugin |
| low | `@openclaw/synthetic-provider` | 341 | 1 | 0 | OpenClaw Synthetic provider plugin |
| low | `@openclaw/web-readability-plugin` | 285 | 1 | 2 | OpenClaw local Readability web extraction plugin |
| low | `@openclaw/qianfan-provider` | 244 | 1 | 0 | OpenClaw Qianfan provider plugin |
| low | `@openclaw/alibaba-provider` | 193 | 2 | 0 | OpenClaw Alibaba Model Studio video provider plugin |
| low | `@openclaw/groq-provider` | 193 | 1 | 0 | OpenClaw Groq media-understanding provider |
| low | `@openclaw/senseaudio-provider` | 173 | 1 | 0 | OpenClaw SenseAudio media-understanding provider |
| low | `@openclaw/sglang-provider` | 170 | 2 | 0 | OpenClaw SGLang provider plugin |
| low | `@openclaw/tencent-provider` | 165 | 0 | 0 | OpenClaw Tencent Cloud provider plugin (TokenHub + Token Plan) |
| low | `@openclaw/tokenjuice` | 162 | 2 | 1 | Bundled tokenjuice exec output compaction plugin |
| low | `@openclaw/copilot-proxy` | 159 | 0 | 0 | OpenClaw Copilot Proxy provider plugin |
| low | `@openclaw/media-understanding-core` | 155 | 0 | 1 | OpenClaw media understanding runtime package |
| low | `@openclaw/cerebras-provider` | 109 | 0 | 0 | OpenClaw Cerebras provider plugin |
| low | `@openclaw/video-generation-core` | 91 | 1 | 0 | OpenClaw video generation runtime package |
| low | `@openclaw/image-generation-core` | 71 | 1 | 0 | OpenClaw image generation runtime package |
| low | `@openclaw/open-prose` | 12 | 0 | 0 | OpenProse VM skill pack plugin (slash command + telemetry). |

## Next-tick port checklist (top-5 `high` by LOC)

- [ ] **memory-core** (`@openclaw/memory-core`) — 48066 LOC, 56 tests, 3 deps. Reason: name matches /memory/. Source: `C:/worktree/openclaw/extensions/memory-core`.
- [ ] **voice-call** (`@openclaw/voice-call`) — 28110 LOC, 45 tests, 4 deps. Reason: name matches /voice/. Source: `C:/worktree/openclaw/extensions/voice-call`.
- [ ] **memory-wiki** (`@openclaw/memory-wiki`) — 15945 LOC, 23 tests, 3 deps. Reason: name matches /memory/. Source: `C:/worktree/openclaw/extensions/memory-wiki`.
- [ ] **memory-lancedb** (`@openclaw/memory-lancedb`) — 4228 LOC, 3 tests, 4 deps. Reason: name matches /memory/. Source: `C:/worktree/openclaw/extensions/memory-lancedb`.
- [ ] **cloudflare-ai-gateway-provider** (`@openclaw/cloudflare-ai-gateway-provider`) — 709 LOC, 3 tests, 0 deps. Reason: name matches /gateway/. Source: `C:/worktree/openclaw/extensions/cloudflare-ai-gateway`.

## Top-5 `medium` (by LOC)

- **discord** — 101608 LOC, 148 tests, 7 deps.
- **telegram** — 87076 LOC, 119 tests, 5 deps.
- **matrix** — 80278 LOC, 118 tests, 8 deps.
- **slack** — 55052 LOC, 95 tests, 6 deps.
- **feishu** — 49865 LOC, 62 tests, 3 deps.

## Top-5 `low` (by LOC)

- **qa-lab** — 65433 LOC, 75 tests, 5 deps.
- **codex** — 64594 LOC, 54 tests, 5 deps.
- **browser-plugin** — 58926 LOC, 104 tests, 6 deps.
- **msteams** — 38279 LOC, 66 tests, 7 deps.
- **qa-matrix** — 28235 LOC, 17 tests, 1 deps.

---

_Generator: `scripts/openclaw-extensions-audit.ts`. Refresh after upstream openclaw sync._
