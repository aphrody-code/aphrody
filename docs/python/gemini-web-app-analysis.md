<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 aphrody contributors -->

# Gemini web app (gemini.google.com/app) — UI inventory & cookie-client feasibility

Analysis of a **live, logged-in** capture of `gemini.google.com/app` (user "Yohan",
session captured headless via CDP with injected Google session cookies — see
`var/forks/gemini_app.png`), cross-referenced against the cookie-authenticated Boq
client `python/aphrody/aphrody/gemini_web.py` and the CLI surface
`python/aphrody/aphrody/cli.py`.

The goal: decide how the keyless cookie path (`aphrody web`) should grow to expose
the features the browser exposes, using the **same** `batchexecute`/`StreamGenerate`
backend — no API key, no OAuth, only the `__Secure-1PSID*` cookie jar managed by
`aphrody.auth.cookies`.

---

## 1. Ground truth: what the client speaks today

`gemini_web.py` does exactly the browser handshake, but only the text-chat slice of it:

1. `GET /app` with the cookie header → scrape `SNlM0e` (the `at` anti-forgery token)
   and `cfb2h` (the `bl` build label) out of `WIZ_global_data`.
2. `POST /_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate`
   with `f.req = [null, "[[prompt], null, [cid,rid,rcid]]"]` and `at=<SNlM0e>`.
3. Parse the `)]}'`-guarded chunked body, take the `wrb.fr` envelope, extract reply +
   `(cid, rid, rcid)` for threading.

The **real wire shape is confirmed** by the captured response dump
(`var/forks/gemini_web_raw.txt`), which the parser in `gemini_web.py` matches precisely:

```
[["wrb.fr",null,"[null,[\"c_…\",\"r_…\"],null,null,
   [[\"rc_…\",[\"Bonjour ! Je suis Gemini…\"],…]],
   [\"Montpellier, France\",\"SWML_DESCRIPTION_FROM_YOUR_INTERNET_ADDRESS\",…],
   …]"]]
```

So `body[1] = [cid, rid]` and `body[4][0] = [rcid, [reply_text], …]` — exactly what
`_extract_reply()` reads. The dump also reveals **side-channel `wrb.fr` envelopes** that
the current parser ignores:

- `{"11":["Présentation de Gemini par Google"]}` — the **auto-generated conversation title**.
- `{"26":"AwAAAAAA…"}` — an opaque continuation/share token.
- `{"46":["c_…",""],"52":[]}` — conversation-id echo + an (empty here) attachments/sources slot.
- Trailing `["di",…]`, `["af.httprm",…]`, `["e",10,…]` — batchexecute framing.

**Key takeaway:** the f.req the client sends has **no model field, no attachment field,
and no tool/mode flag** — those are additional positional slots in the inner array that
the browser fills and we currently leave `null`. That is the seam every enhancement below
threads through.

---

## 2. Screenshot inventory — every visible element

Layout is the current Gemini "app" shell: a slim left icon rail, a centered greeting +
composer, bottom-left account cluster, and one top-right control. UI language is French
(account locale), so labels below are the French strings actually rendered.

### Center
| Element | Label / value | Position |
|---|---|---|
| Greeting | **"Salut Yohan, commençons"** | center, mid-page |
| Composer box | rounded pill input | centered under greeting |
| Composer placeholder | **"Demander à Gemini"** ("Ask Gemini") | left of composer |
| **"+" attach control** | plus glyph | composer far-left — opens add/upload menu (files, images, Drive) |
| **Model selector** | **"Flash-Lite ▾"** (current value = *Flash-Lite*) | composer right side, chevron = dropdown |
| **Mic / voice control** | microphone glyph | composer far-right |

### Left rail (top → bottom)
| Element | Identity | Position |
|---|---|---|
| Gemini logo | rainbow 4-point "spark" | top-left corner |
| **New chat / compose** | pencil-in-circle, **currently highlighted** (active) | upper rail |
| **Search** | magnifying glass | below compose — search across chats |
| **Apps / Gems / Explore** | 2×2 tile grid glyph | below search — Gem manager / app gallery |

### Left rail (bottom cluster)
| Element | Identity | Position |
|---|---|---|
| **History / recent activity** | clock-with-counter-arrow | lower-left |
| **Settings** | gear | below history |
| **Account avatar** | user profile photo (signed-in "Yohan") | bottom-left corner |

### Top-right
| Element | Identity | Position |
|---|---|---|
| **Temporary chat** | pencil inside a dashed/dotted ring | top-right corner — ephemeral, non-persisted session |

---

## 3. Feature-by-feature feasibility on the cookie/Boq path

Classification legend:

- **ALREADY-SUPPORTED** — code already does it.
- **FEASIBLE-SAME-ENDPOINT** — same `StreamGenerate` RPC, just a richer `f.req` (an extra
  positional slot or flag in the inner array). Lowest effort.
- **NEEDS-SEPARATE-RPC** — a different Boq `batchexecute` RPC id under
  `/_/BardChatUi/data/...` (or a Boq upload endpoint). Medium effort, must be discovered.
- **BROWSER-ONLY** — depends on client-side JS / device APIs; not reachable from an HTTP client.

| # | Feature (UI element) | Class | Notes / what to drive it |
|---|---|---|---|
| 1 | **Multi-turn threading** (new-chat icon, implicit) | **ALREADY-SUPPORTED** | `keep_context=True` round-trips `[cid,rid,rcid]` in slot 3 of the inner f.req; verified live (`c_…`,`r_…`,`rc_…` echo back). `reset()` starts a fresh thread. CLI `web` currently forces `keep_context=False` — a one-liner away from threaded. |
| 2 | **Text generation** (composer) | **ALREADY-SUPPORTED** | The core path. |
| 3 | **Model selection** ("Flash-Lite ▾") | **FEASIBLE-SAME-ENDPOINT** | The browser does *not* change RPC per model — it sends a **model tag in the f.req** (and/or an `X-Goog-*` / `f.sid`-adjacent param). Our inner array currently stops at slot 3; the model selector value ("Flash-Lite", "2.5 Pro", "Deep Research"…) maps to an opaque model key the page carries in `WIZ_global_data`. **Investigate:** diff two `StreamGenerate` f.req bodies (Flash-Lite vs Pro) — the differing positional element is the model field. Also scrape the model→key map from the bootstrap HTML alongside `SNlM0e`/`cfb2h`. |
| 4 | **File / image upload** ("+" attach) | **NEEDS-SEPARATE-RPC** | Browser uploads bytes to a **Boq push/scotty upload endpoint** first (returns an opaque blob/file id), then references that id in a later `StreamGenerate` f.req slot. **Investigate:** the `push.clients6.google.com` / `/_/BardChatUi/data/.../upload` (Scotty resumable) handshake and which inner-array slot carries the returned id. The `{"52":[]}` slot in the live dump is a strong candidate for the attachment/source list. |
| 5 | **Voice input** (mic glyph) | **BROWSER-ONLY** (input capture) → audio attach is **NEEDS-SEPARATE-RPC** | The mic is Web Speech / MediaRecorder in-page — not an HTTP feature. *However*, aphrody already owns offline STT (`cli.voice` → Whisper in `voice_server.py`): transcribe locally, then send text via path #2. Sending **recorded audio** as an attachment would reuse the upload RPC (#4). Recommend the local-STT bridge, not reverse-engineering the mic. |
| 6 | **Image generation** (via prompt / Gems) | **FEASIBLE-SAME-ENDPOINT** *(web)* / **ALREADY-SUPPORTED** *(Vertex)* | In the web app, "generate an image" is a normal `StreamGenerate` turn whose response carries an image part (look for an inline-data / image-url slot in `body[4][0]`, sibling to the text slot). aphrody **already** generates images keylessly via the *other* surface — `images.py`/`NanoBanana` over Vertex (`gemini-2.5-flash-image`). For the cookie path specifically, parsing the image-bearing response slot is the only new work; prefer the existing Vertex path for reliability. |
| 7 | **Deep Research / grounding** | **FEASIBLE-SAME-ENDPOINT** (mode flag) + **NEEDS richer parse** | Deep Research is a **mode toggle** selected near the model picker; it rides the same RPC with a mode/tool flag in the f.req, then streams multiple `wrb.fr` chunks (plan → sources → answer). Grounding/citations already appear in the live response (the `["Montpellier, France","SWML_DESCRIPTION_FROM_YOUR_INTERNET_ADDRESS",…]` block in `body[5]`, plus the `{"26":…}` token). **Investigate:** the mode flag's positional slot and parse `body[5]`/`{"52"}` for source URLs. |
| 8 | **Gems / Apps** (2×2 grid icon) | **NEEDS-SEPARATE-RPC** | Listing/selecting a Gem (custom system-prompt persona) and the Gem gallery are their own Boq RPCs (list-gems / get-gem). Once a Gem is chosen, conversing with it is again `StreamGenerate` with a **gem-id slot** in the f.req. **Investigate:** the list RPC id; then the gem-id inner slot. |
| 9 | **Chat history listing** (clock icon) | **NEEDS-SEPARATE-RPC** | The recent-chats panel is populated by a dedicated list RPC (historically a `…BardFrontendService/ListConversations`-style id under the same `/_/BardChatUi/data/` mount), returning `(cid, title, snippet, ts)` tuples. The titles are exactly the `{"11":[…]}` strings we already see generated per turn. **Investigate:** capture the panel's `batchexecute` request; the RPC id is the 6-char rpcid in its `f.req`. |
| 10 | **Search across chats** (magnifier) | **NEEDS-SEPARATE-RPC** | A query RPC over the conversation index; depends on #9 existing. Lower priority. |
| 11 | **Settings** (gear) | **BROWSER-ONLY** (mostly) | Account/preferences UI; a few toggles (e.g. activity) have RPCs but offer little CLI value. Skip. |
| 12 | **Temporary chat** (top-right dotted-pencil) | **FEASIBLE-SAME-ENDPOINT** | Ephemeral = "do not persist + do not thread." We **already** get this for free by calling `generate(..., keep_context=False)` and never reusing the ids. Effectively a flag, not an RPC. |
| 13 | **Account / avatar** (bottom-left) | **ALREADY-SUPPORTED (adjacent)** | Identity is implicit in the cookie jar; `aphrody whoami` already reports the signed-in account (via the OAuth surface). No web-path work. |

---

## 4. Prioritized recommendations (keyless cookie client)

Ordered by value-to-effort. Every item stays inside the existing cookie/Boq seam — no API
key, no OAuth — and names the concrete f.req slot or Boq RPC to investigate first.

### P0 — Thread the CLI through existing capability (hours, zero RE)
1. **`aphrody web --thread` / a `web chat` REPL.** The client already threads; the CLI just
   hard-codes `keep_context=False`. Expose a flag (and optionally print the `conversation`
   ids so a follow-up command can resume). *No new wire work — `GeminiWebClient.generate`
   already round-trips `(cid,rid,rcid)`.*
2. **Capture the conversation title.** Parse the `{"11":[…]}` side `wrb.fr` envelope (already
   present in `gemini_web_raw.txt`) and surface it. Trivial parser addition; immediately
   useful for naming/threading.

### P1 — `--model` flag mapped to the web picker (small RE, high value)
3. **`aphrody web --model <flash-lite|2.5-pro|…>`.** *Investigate:* diff two real
   `StreamGenerate` `f.req` bodies that differ only by the picker value to locate the **model
   slot** in the inner array; in parallel, scrape the **model→key table** from the bootstrap
   `/app` HTML (next to `SNlM0e`/`cfb2h` in `WIZ_global_data`). Wire a `MODEL_ALIASES` map →
   inject the key into the f.req. Same `StreamGenerate` RPC; no new endpoint.

### P2 — Conversations list command (one new RPC)
4. **`aphrody web conversations` (list) + `web resume <cid>`.** *Investigate:* the
   recent-chats panel's `batchexecute` call — its 6-char **rpcid** under
   `/_/BardChatUi/data/assistant.lamda.BardFrontendService/…` (the `ListConversations`-class
   id). Reuse the existing `at`/`bl` handshake and `)]}'` chunk parser; map rows to
   `(cid, title, snippet, ts)`. Pairs naturally with the title parse from P0.

### P3 — File/image attach via the upload RPC (most RE, unlocks multimodal)
5. **`aphrody web --attach <path>`.** *Investigate:* the **Scotty/Boq resumable upload**
   handshake the "+" menu uses (candidate hosts: `push.clients6.google.com`, or an
   `…/upload` mount under `/_/BardChatUi/data/`) → capture the returned blob/file id → place
   it in the attachment slot of the `StreamGenerate` f.req (the `{"52":[]}` slot is the prime
   suspect from the live dump). Unlocks image understanding and document Q&A on the keyless
   path.

### Deferred / explicitly not worth it
- **Voice input:** bridge through aphrody's existing offline Whisper (`cli.voice`) → text →
  path #2. Do **not** reverse-engineer the in-page mic (BROWSER-ONLY).
- **Image *generation* on the cookie path:** aphrody already ships keyless image generation
  via Vertex (`images.py`/`NanoBanana`); only parse the web image-response slot if a
  cookie-only image path is specifically required.
- **Settings, cross-chat search, Gems gallery:** low CLI value or dependent on P2; defer.
- **Deep Research:** feasible as a mode flag on `StreamGenerate`, but parsing the multi-chunk
  plan/sources stream is non-trivial — schedule after P1/P2 land and the parser is
  generalized to walk all `wrb.fr` envelopes (not just the text one).

---

## 5. One structural note for the implementer

Every P1–P3 item widens the **same inner f.req array** the client builds in
`GeminiWebClient.generate`:

```python
inner = json.dumps([[prompt], None, context])   # slots: [0]=prompt, [1]=?, [2]=context
```

The browser populates **further positional slots** of that array for model, mode (Deep
Research), and attachment ids, and emits **multiple** `wrb.fr` envelopes (text, title `"11"`,
sources/`"52"`, token `"26"`). The current parser keeps only the text envelope. So the
enabling refactor for *all* of the above is: (a) make the inner-array builder slot-addressable
instead of a fixed 3-tuple, and (b) generalize `_parse_stream` to return **every** decoded
`wrb.fr` envelope keyed by its dict keys, not just the candidate text. Land that, and
model/title/sources/attachments become incremental field reads rather than separate rewrites.

*(No code was modified in producing this report; it is analysis only.)*
