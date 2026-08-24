// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Resident-model backend: one `llama-server`, many pages.
//
// The CLI backend (`vlm.rs`) spawns `llama-mtmd-cli` per page, which reloads
// several gigabytes of weights every time. Measured on a Dragon Ball databook
// lot: 7.6 s per plate, of which a large share is load, not inference. Over
// eleven thousand plates that is most of a day spent re-reading the same file.
//
// This module keeps the model resident behind `llama-server` and posts each
// page to its OpenAI-compatible endpoint. The trade is the one the CLI backend
// was chosen to avoid — a crash takes the whole run rather than one page — so
// the server is supervised: its health is checked before the first page, and a
// dead server is reported as such instead of failing every remaining plate
// with a confusing transport error.
//
// The process is owned by `ServerRunner` and killed on drop, so an interrupted
// batch does not leave a multi-gigabyte process holding the GPU.
//
// WHAT IT COSTS, MEASURED (dots.ocr, RTX 4070, twelve databook plates)
//
// Speed: 4.9 s per plate against 13 s for the CLI backend — a little under 3×,
// because the CLI reloads four gigabytes of weights for every single page.
//
// Fidelity used to be the objection, and it was the wrong diagnosis. On plate
// 18-0249 this backend returned 1384 characters where `llama-mtmd-cli`
// returned 1624, stopping before the folio; raising the budget from 1024 to
// 3072 changed nothing, which was read at the time as "the same weights see
// the image differently through the two front ends".
//
// The server's own logs say otherwise, and they were sitting in the temp
// directory the whole time. `n_ctx_slot = 131072`, prompt 1936 tokens,
// `truncated = 0` on every request — nothing was ever cut for want of context.
// What the logs show instead is six plates out of twelve stopping at EXACTLY
// 1024 generated tokens, and three identical runs of the same lot producing
// byte-identical output. The server was not stopping early: it was not
// stopping at all, and the budget was cutting it off mid-loop.
//
// Two causes, both now fixed and both shared with the CLI backend:
//
//   * the model's end-of-turn token is not in llama.cpp's end-of-generation
//     set, so nothing hears the model finish — see `vlm::eot_override`;
//   * the two front ends disagree on the chat template, because jinja is on by
//     default here and off in `llama-mtmd-cli` — see the `--jinja` flag there.
//
// This backend is still `--server`, opt-in, until the fixes are measured on
// real plates. But the reason is now "not yet re-measured", not "reads less".

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use aphrody_infer::llama::{self, LlamaTool};

use crate::doctags::Document;
use crate::vlm::{OcrOptions, PageResult, PageText, resolve_artifacts};
use crate::{OcrError, Result};

/// How long to wait for the server to answer its health endpoint.
///
/// Loading a 4 GiB GGUF onto the GPU takes tens of seconds on a cold page
/// cache; a minute is generous without hanging a batch forever on a server
/// that will never come up.
const STARTUP_TIMEOUT: Duration = Duration::from_mins(2);

/// Per-request timeout for a server serving one request at a time.
///
/// A dense plate takes seconds, not minutes; beyond this it is a stuck
/// generation, and failing the page beats stalling the batch behind it.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(3);

/// What each extra slot adds to that budget.
///
/// A request does not only wait for its own decode: with N slots in flight it
/// waits behind the others sharing the same batch. Three minutes flat was
/// calibrated on 1000x1495 plates, and it held — until a re-read of 2048x1526
/// scans, four times the visual tokens, at eight slots. Requests then failed
/// with `os error 10060` on plates that were generating perfectly well; the
/// timeout was reading contention as a hang.
///
/// Scaling with the slot count keeps the guarantee that matters — a genuinely
/// stuck generation still fails rather than hanging the batch forever — while
/// not calling a queued request stuck.
const REQUEST_TIMEOUT_PER_SLOT: Duration = Duration::from_secs(45);

/// How a resident server is sized.
#[derive(Debug, Clone, Copy)]
pub struct ServerConfig {
    /// Loopback port to bind. Never exposed off the machine: the server holds
    /// no authentication of any kind.
    pub port: u16,
    /// Requests the server serves at once, one slot each.
    ///
    /// This is where the throughput is. Generating a page is bound by memory
    /// bandwidth, not by arithmetic: at one sequence, the GPU reads four
    /// gigabytes of weights to produce a single token, then reads them again
    /// for the next one. Two sequences in flight read those same weights once
    /// and produce two tokens — the second page is very nearly free. The cost
    /// is one KV cache per slot, not one copy of the model per slot.
    pub slots: u32,
    /// Context window granted to each slot, in tokens.
    ///
    /// `llama-server` divides `--ctx-size` between its slots, so the flag is
    /// computed as `slots × this`. Sized per slot rather than globally because
    /// what a page needs does not shrink when a second page runs beside it —
    /// and a slot too small for a dense plate truncates the transcription
    /// silently.
    pub ctx_per_slot: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 8791, slots: DEFAULT_SLOTS, ctx_per_slot: DEFAULT_CTX_PER_SLOT }
    }
}

/// Slots opened when nothing else is asked.
///
/// Four, and the number came from re-measuring on the plates this pipeline is
/// actually pointed at. An earlier default of two rested on a light batch —
/// 1600x1056 images, some 250 tokens generated each — where the image prompt
/// dominated and extra slots queued against each other. The databook corpus is
/// nothing like that: 1340x2048 plates generating 1357 tokens, so the 2.3 s
/// image prompt is 15 % of a page and the 13.5 s of decode is the rest. Decode
/// is bound by memory bandwidth, so a second sequence is nearly free, and the
/// curve inverts — twelve real plates took 196 s at two slots, 146 s at four,
/// 118 s at eight.
///
/// Eight is therefore faster still on the 12 GiB card this was measured on,
/// but each slot costs a full KV cache on top of a 4.1 GiB model; four is what
/// a smaller card can also hold. Ask for more when the card allows it.
///
/// What the number is *not* is a fidelity setting. Two runs with identical
/// slots, `temperature 0` and a pinned seed still diverge on seven plates out
/// of twelve: llama.cpp's batched attention kernels are not numerically
/// invariant to batch composition, and that composition depends on the order
/// pages land in slots. Picking a slot count "for stable output" buys nothing.
pub const DEFAULT_SLOTS: u32 = 4;

/// Context per slot when nothing else is asked.
///
/// A vision model spends most of its context on the image, not on the answer:
/// a high-resolution plate becomes thousands of visual tokens before the first
/// word is generated. llama.cpp's own default of 4096 leaves too little room
/// after that for a dense databook page, and the failure is silent — the text
/// simply stops early, which is indistinguishable from a page that had nothing
/// more to say.
pub const DEFAULT_CTX_PER_SLOT: u32 = 8192;

/// A running `llama-server` with a vision model loaded.
pub struct ServerRunner {
    child: Child,
    config: ServerConfig,
    options: OcrOptions,
}

impl ServerRunner {
    /// Start a server for the model named by `options` and wait until it is
    /// ready to answer.
    ///
    /// The port is bound on loopback only: this server holds no auth and must
    /// not be reachable from the network.
    ///
    /// # Errors
    ///
    /// [`OcrError::NoRunner`] when llama.cpp is not installed,
    /// [`OcrError::Model`] when the weights are not pulled, and
    /// [`OcrError::Process`] when the server does not become healthy.
    pub fn start(options: OcrOptions, config: ServerConfig) -> Result<Self> {
        let source = llama::resolve(LlamaTool::Server).ok_or(OcrError::NoRunner)?;
        let (weights, mmproj) = resolve_artifacts(&options.model_id)?;
        let port = config.port;

        let mut command = Command::new(source.path());
        command
            .arg("-m")
            .arg(&weights)
            .arg("--mmproj")
            .arg(&mmproj)
            .arg("-ngl")
            .arg(options.gpu_layers.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            // One slot per page in flight. The weights are read once for the
            // whole batch of sequences, so a second page costs a KV cache and
            // almost no extra time.
            .arg("--parallel")
            .arg(config.slots.to_string())
            // Divided between the slots by llama-server, hence the product.
            // Sized so a dense plate's visual tokens and its answer both fit:
            // an undersized context truncates the transcription without
            // saying so.
            //
            // Explicit rather than left to `--fit`, which picked 131072 on
            // seven launches and 8192 on fourteen others of the same machine.
            // A context that changes with whatever else holds VRAM is a
            // reproducibility hole, and 131072 tokens of KV cache costs 3.5
            // GiB for a prompt that never exceeds two thousand.
            .arg("-c")
            .arg((config.slots * config.ctx_per_slot).to_string())
            // Never let the server drop the head of a conversation to make
            // room: a silently shifted context is a silently wrong page.
            .arg("--no-context-shift")
            // The chat parser exists to lift reasoning traces out of an
            // answer. There is no reasoning here, only a transcription, and
            // any rewriting of `message.content` is damage.
            .arg("--reasoning-format")
            .arg("none")
            .arg("--skip-chat-parsing")
            .stdout(Stdio::null());

        // The same end-of-turn token the CLI backend installs. Both front ends
        // must see the same vocabulary, or one of them stops where the other
        // does not.
        if let Some(kv) = crate::vlm::eot_argument(options.eot_token) {
            command.arg("--override-kv").arg(kv);
        }

        // llama-server logs continuously on stderr. Piping it without ever
        // reading the pipe deadlocks the server the moment the OS buffer fills
        // (64 KiB on Windows) — it blocks on write and stops answering. So the
        // log goes to a file: never blocks, and still readable when the server
        // dies before becoming healthy.
        let log_path = std::env::temp_dir().join(format!("aphrody-llama-server-{port}.log"));
        let log = std::fs::File::create(&log_path)
            .map_err(|source| OcrError::Io { path: log_path.clone(), source })?;
        command.stderr(Stdio::from(log));

        let mut child = command
            .spawn()
            .map_err(|source| OcrError::Io { path: source_path(&source, port), source })?;

        if let Err(e) = wait_healthy(port, &mut child, &log_path) {
            // Never leave a half-started server holding the GPU.
            let _ = child.kill();
            return Err(e);
        }

        Ok(Self { child, config, options })
    }

    /// The endpoint this runner talks to.
    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.config.port)
    }

    /// How many pages this server can read at once.
    ///
    /// A caller driving the batch reads this rather than assuming: sending
    /// more requests than there are slots does not go faster, it queues.
    #[must_use]
    pub const fn slots(&self) -> u32 {
        self.config.slots
    }

    /// Per-request timeout, widened by how many requests share the batch.
    ///
    /// See [`REQUEST_TIMEOUT_PER_SLOT`]: a queued request is not a stuck one.
    fn request_timeout(&self) -> Duration {
        REQUEST_TIMEOUT + REQUEST_TIMEOUT_PER_SLOT * self.config.slots.saturating_sub(1)
    }

    /// Turn a server refusal into something a caller can act on.
    ///
    /// See [`explique_refus`].
    fn explique(&self, body: &str) -> String {
        explique_refus(body, self.config.ctx_per_slot)
    }


    /// Read one image through the resident model.
    ///
    /// # Errors
    ///
    /// [`OcrError::Io`] when the image cannot be read, [`OcrError::Process`]
    /// when the server rejects the request or answers unusably.
    pub fn read(&self, image: &Path) -> Result<PageResult> {
        let started = Instant::now();
        let data_uri = data_uri(image)?;

        let body = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "image_url", "image_url": { "url": data_uri } },
                    { "type": "text", "text": self.options.prompt },
                ],
            }],
            "max_tokens": self.options.max_tokens,
            // Greedy, because there is one right reading of a plate and no
            // reason to sample away from it.
            //
            // Ce que ce zéro ne donne PAS, contrairement à ce qui était écrit
            // ici : la reproductibilité. Mesuré le 2026-08-23 — deux runs
            // strictement identiques, mêmes douze planches, même `--slots 4`,
            // même échantillonnage épinglé, `seed 0` — divergent sur SEPT
            // planches sur douze, dont une à 0,213 de ressemblance. Le décodage
            // glouton n'est déterministe que pour une composition de lot
            // donnée ; celle-ci dépend de l'ordre d'arrivée des pages dans les
            // slots, et les noyaux d'attention batchés de llama.cpp ne sont pas
            // numériquement invariants à la taille du lot.
            //
            // Conséquence à connaître avant de s'appuyer sur ce backend : une
            // planche peut ressortir à 47 caractères là où un autre passage en
            // rend 775 — arrêt précoce, pas contenu absent. Le dépôt reste
            // idempotent parce qu'il est en `mode: "merge"`, pas parce que la
            // lecture serait stable.
            "temperature": 0.0,
            // Sampling is pinned rather than left to the server's defaults,
            // which differ from the CLI's (temperature 0.80 against 0.20) and
            // could change with a llama.cpp release. At temperature 0 the
            // decode is greedy and none of these can alter the argmax — but
            // the repetition penalties act on the logits BEFORE temperature,
            // so a non-zero one would genuinely move it. Spelling them out
            // makes the request say what it does instead of inheriting it.
            "top_k": 0,
            "top_p": 1.0,
            "min_p": 0.0,
            "typical_p": 1.0,
            "repeat_penalty": 1.0,
            "presence_penalty": 0.0,
            "frequency_penalty": 0.0,
            "dry_multiplier": 0.0,
            "seed": 0,
            // Le cache reste désactivé pour une raison qui, elle, tient. The
            // server reuses one slot's KV cache across requests, so a plate
            // read second could come out differently from the same plate read
            // first — measured: paragraph breaks collapsed to spaces. A batch
            // resumed at another offset would then rewrite text it had already
            // deposited. The saved prefix is a few hundred tokens; correctness
            // is worth more.
            "cache_prompt": false,
            "stream": false,
        });

        let payload = serde_json::to_string(&body).map_err(|e| OcrError::Process {
            command: self.endpoint(),
            status: "serialise request".to_owned(),
            stderr: e.to_string(),
        })?;

        let response = crate::http::post_json(
            self.config.port,
            "/v1/chat/completions",
            &payload,
            self.request_timeout(),
        )?;
        if !response.is_success() {
            return Err(OcrError::Process {
                command: self.endpoint(),
                status: response.status.to_string(),
                stderr: self.explique(&response.body),
            });
        }

        let content = extract_content(&response.body).ok_or_else(|| OcrError::Process {
            command: self.endpoint(),
            status: "malformed response".to_owned(),
            stderr: crate::vlm::tail(&response.body, 400),
        })?;

        let document = Document::parse(&content);
        let page_text = document
            .to_markdown()
            .map_or(PageText::None, |markdown| PageText::Text { markdown });

        Ok(PageResult {
            image: image.to_path_buf(),
            text: page_text,
            elapsed_ms: started.elapsed().as_millis(),
            raw: self.options.keep_raw.then_some(content),
        })
    }
}

impl Drop for ServerRunner {
    fn drop(&mut self) {
        // A resident model holds gigabytes of VRAM; leaking it past the batch
        // would make the next run fail for want of memory.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Poll the server until it reports ready, or the child dies, or time runs out.
fn wait_healthy(port: u16, child: &mut Child, log_path: &Path) -> Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;

    while Instant::now() < deadline {
        // A server that exited will never become healthy: say so immediately
        // rather than burning the whole timeout.
        if let Ok(Some(status)) = child.try_wait() {
            let stderr = std::fs::read_to_string(log_path).unwrap_or_default();
            return Err(OcrError::Process {
                command: "llama-server".to_owned(),
                status: status.to_string(),
                stderr: crate::vlm::tail(&stderr, 600),
            });
        }

        if let Ok(response) = crate::http::get(port, "/health", Duration::from_secs(2)) {
            if response.is_success() {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err(OcrError::Process {
        command: "llama-server".to_owned(),
        status: "timeout".to_owned(),
        stderr: format!("no healthy answer from 127.0.0.1:{port} within {STARTUP_TIMEOUT:?}"),
    })
}

/// Read an image and encode it as a `data:` URI for the chat API.
fn data_uri(image: &Path) -> Result<String> {
    let bytes = std::fs::read(image)
        .map_err(|source| OcrError::Io { path: image.to_path_buf(), source })?;
    Ok(format!("data:{};base64,{}", mime_for(image), base64(&bytes)))
}

/// MIME type from the file extension, defaulting to JPEG.
#[must_use]
pub fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        // The databook export writes JPEG, and so does almost every scan.
        _ => "image/jpeg",
    }
}

/// Standard base64, no line breaks.
///
/// Hand-rolled rather than pulling a crate: it is twenty lines, it has one
/// caller, and the alternative is another entry in the supply chain for an
/// encoding that has not changed since 1987.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 { ALPHABET[(triple & 0x3F) as usize] as char } else { '=' });
    }
    out
}

/// Pull `choices[0].message.content` out of a chat-completions response.
#[must_use]
pub fn extract_content(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()
        .map(ToOwned::to_owned)
}

/// Path to report when spawning the server fails.
fn source_path(_error: &std::io::Error, _port: u16) -> PathBuf {
    llama::resolve(LlamaTool::Server)
        .map_or_else(|| PathBuf::from("llama-server"), |s| s.path().to_path_buf())
}

/// Turn a server refusal into something a caller can act on.
///
/// One refusal deserves it: a plate whose image alone outgrows the slot's
/// context. The server says so precisely — `exceed_context_size_error`, with
/// the token counts — but relayed as a bare 400 it reads like any other server
/// error, and a run keeps losing every large plate in silence. Measured on
/// full-resolution scans: 14 300 tokens of image against an 8192-token slot,
/// on thirty-two plates in a row.
///
/// The fix belongs to the caller: `--ctx` is theirs to raise, and raising it
/// costs KV cache we cannot spend on their behalf. So say what to do, and
/// leave the deciding.
fn explique_refus(body: &str, ctx_per_slot: u32) -> String {
    if body.contains("exceed_context_size_error") {
        let jetons = body
            .split("\"n_prompt_tokens\":")
            .nth(1)
            .and_then(|reste| reste.split(|c: char| !c.is_ascii_digit()).find(|s| !s.is_empty()))
            .unwrap_or("?");
        let suggere = jetons
            .parse::<u32>()
            .map_or(32768, |n| n.saturating_mul(2).next_power_of_two().max(16384));
        return format!(
            "the image alone needs {jetons} tokens, more than this slot's              {ctx_per_slot} — re-run with `--ctx {suggere}` (each slot costs              that much KV cache)"
        );
    }
    crate::vlm::tail(body, 400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_test_vectors() {
        // RFC 4648 §10.
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_handles_bytes_above_ascii() {
        // A JPEG is mostly high bytes; a signed-char bug would corrupt them.
        assert_eq!(base64(&[0xFF, 0xD8, 0xFF]), "/9j/");
        assert_eq!(base64(&[0x00, 0x00, 0x00]), "AAAA");
        assert_eq!(base64(&[0xFF; 6]), "////////");
    }

    #[test]
    fn base64_length_is_always_a_multiple_of_four() {
        for len in 0..32_usize {
            let encoded = base64(&vec![0xA5; len]);
            assert_eq!(encoded.len() % 4, 0, "len {len} -> {encoded}");
        }
    }

    #[test]
    fn mime_follows_the_extension_case_insensitively() {
        assert_eq!(mime_for(Path::new("a.png")), "image/png");
        assert_eq!(mime_for(Path::new("a.PNG")), "image/png");
        assert_eq!(mime_for(Path::new("a.webp")), "image/webp");
        assert_eq!(mime_for(Path::new("a.jpg")), "image/jpeg");
        // Unknown or absent extension falls back to JPEG, what scans are.
        assert_eq!(mime_for(Path::new("a")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.tiff")), "image/jpeg");
    }

    #[test]
    fn content_is_extracted_from_a_chat_completion() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"読めた"}}]}"#;
        assert_eq!(extract_content(body).as_deref(), Some("読めた"));
    }

    #[test]
    fn a_malformed_response_yields_none_rather_than_panicking() {
        for body in [
            "",
            "not json",
            "{}",
            r#"{"choices":[]}"#,
            r#"{"choices":[{}]}"#,
            r#"{"choices":[{"message":{}}]}"#,
            r#"{"choices":[{"message":{"content":null}}]}"#,
            r#"{"error":{"message":"context too long"}}"#,
        ] {
            assert_eq!(extract_content(body), None, "{body}");
        }
    }

    #[test]
    fn timeouts_are_bounded_and_ordered() {
        // Startup may be slow; a single request must never be slower than the
        // batch can tolerate.
        assert!(STARTUP_TIMEOUT >= Duration::from_mins(1));
        assert!(REQUEST_TIMEOUT >= Duration::from_mins(1));
    }

    #[test]
    fn un_refus_de_contexte_dit_quoi_faire() {
        let runner_config = ServerConfig { port: 8791, slots: 2, ctx_per_slot: 8192 };
        let corps = r#"{"error":{"code":400,"message":"request (14368 tokens) exceeds the available context size (8192 tokens), try increasing it","type":"exceed_context_size_error","n_prompt_tokens":14368,"n_ctx":8192}}"#;
        let explique = explique_refus(corps, runner_config.ctx_per_slot);
        assert!(explique.contains("14368"), "le compte de jetons doit survivre : {explique}");
        assert!(explique.contains("--ctx"), "il faut dire quoi faire : {explique}");
        assert!(explique.contains("32768"), "et suggerer une valeur qui passe : {explique}");
    }

    #[test]
    fn une_erreur_ordinaire_passe_telle_quelle() {
        let corps = "internal server error, something else entirely";
        let explique = explique_refus(corps, 8192);
        assert!(explique.contains("something else entirely"));
        assert!(!explique.contains("--ctx"), "ne pas suggerer --ctx a tort : {explique}");
    }

    #[test]
    fn le_budget_par_requete_suit_le_nombre_de_slots() {
        let budget = |slots| {
            REQUEST_TIMEOUT + REQUEST_TIMEOUT_PER_SLOT * (u32::from(slots) - 1)
        };
        // Un seul slot : rien a partager, le budget de base suffit.
        assert_eq!(budget(1_u8), REQUEST_TIMEOUT);
        // Huit slots sur des scans lourds : c'est le cas qui rendait
        // `os error 10060` sur des planches qui generaient tres bien.
        assert!(budget(8_u8) >= Duration::from_mins(8));
        assert!(budget(2_u8) > budget(1_u8), "le budget doit croitre avec les slots");
    }
}
