// SPDX-License-Identifier: Apache-2.0
//! `aphrody hermes` — a multi-channel, voice-to-voice agent runtime.
//!
//! Positioned to surpass NousResearch's `hermes-agent` on three concrete axes:
//!
//! 1. **X / Twitter channel** — `hermes-agent` ships Telegram/Discord/Slack/ WhatsApp/Signal/Email
//!    but **no X**; aphrody drives X via [`aphrody_messaging::channels::x`] (cookie-auth
//!    `aphrody-x` binary).
//! 2. **Voice-to-voice (full duplex)** — `hermes-agent` only *transcribes* voice memos (one-way).
//!    aphrody closes the loop: inbound audio → STT ([`aphrody_voice::stt`]) → chat → TTS
//!    ([`aphrody_voice`]) → outbound audio.
//! 3. **Gemini 3.5 Flash** — the chat turn runs on Gemini 3.5 Flash (the `aphrody-chat` default),
//!    keyless via the agy / web transports.
//!
//! ## Architecture
//!
//! The orchestration core [`process_inbound`] is written against four narrow,
//! **object-safe** capability traits ([`ChatEngine`], [`Transcriber`],
//! [`Speaker`], [`Sink`]) so it can be unit-tested end-to-end with stubs and
//! reused across every channel without touching the (non-object-safe)
//! `MessagingChannel` / `SttProvider` / `VoiceProvider` traits directly. Thin
//! adapters bridge the real implementations onto these traits.

use std::path::Path;

use aphrody_messaging::channels::{Attachment, InboundMessage, MessagingChannel};
use async_trait::async_trait;
use thiserror::Error;

// ── Errors ──────────────────────────────────────────────────────────────────

/// Failure surfaced while processing one inbound message.
#[derive(Debug, Error)]
pub(crate) enum AgentError {
    /// The chat backend failed to produce a reply.
    #[error("chat engine error: {0}")]
    Chat(String),
    /// Speech-to-text failed.
    #[error("transcription error: {0}")]
    Stt(String),
    /// Text-to-speech failed.
    #[error("synthesis error: {0}")]
    Tts(String),
    /// The outbound channel rejected a reply.
    #[error("sink error: {0}")]
    Sink(String),
    /// An inbound audio attachment could not be fetched.
    #[error("attachment fetch error: {0}")]
    Fetch(String),
}

// ── Narrow, object-safe capability traits ────────────────────────────────────

/// Produces an assistant reply for a user turn. Wraps the chat turn loop.
#[async_trait]
pub(crate) trait ChatEngine: Send {
    /// Run one user→assistant turn and return the assistant's text.
    async fn reply(&mut self, user_text: &str) -> Result<String, AgentError>;
}

/// Speech-to-text: decode an audio payload into text.
#[async_trait]
pub(crate) trait Transcriber: Send + Sync {
    /// Transcribe `audio` (raw bytes of a media file) into UTF-8 text.
    async fn to_text(&self, audio: &[u8], mime: &str) -> Result<String, AgentError>;
}

/// Text-to-speech: render text into an audio payload (with its MIME type).
#[async_trait]
pub(crate) trait Speaker: Send + Sync {
    /// Synthesise `text` into audio bytes, returning `(bytes, mime_type)`.
    async fn to_audio(&self, text: &str) -> Result<(Vec<u8>, String), AgentError>;
}

/// Outbound channel sink: how the agent replies.
#[async_trait]
pub(crate) trait Sink: Send + Sync {
    /// Send a text reply to `room`.
    async fn reply_text(&self, room: &str, text: &str) -> Result<(), AgentError>;
    /// Send an audio reply to `room` (voice-to-voice outbound leg).
    async fn reply_audio(&self, room: &str, audio: &[u8], mime: &str) -> Result<(), AgentError>;
}

// ── Config + result ───────────────────────────────────────────────────────────

/// Runtime knobs for the agent loop.
#[derive(Debug, Clone)]
pub(crate) struct AgentConfig {
    /// When `true`, inbound audio is transcribed and replies are also voiced
    /// (the full voice-to-voice loop). When `false`, the agent is text-only.
    pub(crate) voice: bool,
    /// Only act on messages whose text contains this trigger (e.g. a mention).
    /// Empty string = respond to everything.
    pub(crate) trigger: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { voice: false, trigger: String::new() }
    }
}

/// Outcome of processing one inbound message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentTurn {
    /// The user text the engine actually saw (transcribed, if audio).
    pub(crate) user_text: String,
    /// The assistant reply text.
    pub(crate) reply: String,
    /// Whether the reply was also delivered as synthesised audio.
    pub(crate) voiced: bool,
    /// Whether the inbound leg was transcribed from audio.
    pub(crate) from_audio: bool,
    /// `true` when the message was skipped (trigger not matched / empty input).
    pub(crate) skipped: bool,
}

/// First audio attachment on a message, if any (`audio/*` MIME).
#[must_use]
pub(crate) fn first_audio<'a>(msg: &'a InboundMessage) -> Option<&'a Attachment> {
    msg.attachments.iter().find(|a| a.mime_type.starts_with("audio/"))
}

// ── Orchestration core (the tested heart) ─────────────────────────────────────

/// Process a single inbound message end-to-end.
///
/// Pipeline:
/// 1. **Resolve input** — if `cfg.voice` and the message carries an `audio/*` attachment with a
///    transcriber, fetch+transcribe it; otherwise use `msg.text`.
/// 2. **Trigger gate** — skip if `cfg.trigger` is non-empty and absent from the resolved text.
/// 3. **Chat** — run one turn through `chat`.
/// 4. **Reply** — `sink.reply_text`; and when `cfg.voice` + `speaker`, also synthesise and
///    `sink.reply_audio` (full voice-to-voice).
///
/// `fetch_audio` is injected so the network fetch is stubbable in tests.
///
/// # Errors
/// Surfaces [`AgentError`] from any stage (transcription, chat, synthesis, sink).
pub(crate) async fn process_inbound<F, Fut>(
    msg: &InboundMessage,
    chat: &mut dyn ChatEngine,
    transcriber: Option<&dyn Transcriber>,
    speaker: Option<&dyn Speaker>,
    sink: &dyn Sink,
    cfg: &AgentConfig,
    fetch_audio: F,
) -> Result<AgentTurn, AgentError>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>, AgentError>>,
{
    // 1. Resolve the user text (transcribe inbound audio when enabled).
    let mut from_audio = false;
    let user_text = if cfg.voice {
        if let (Some(att), Some(stt)) = (first_audio(msg), transcriber) {
            let url = att.url.clone().ok_or_else(|| {
                AgentError::Fetch(format!("attachment {} has no download URL", att.id))
            })?;
            let bytes = fetch_audio(url).await?;
            from_audio = true;
            stt.to_text(&bytes, &att.mime_type).await?
        } else {
            msg.text.clone()
        }
    } else {
        msg.text.clone()
    };

    let user_text = user_text.trim().to_owned();

    // 2. Trigger gate / empty guard.
    if user_text.is_empty() || (!cfg.trigger.is_empty() && !user_text.contains(&cfg.trigger)) {
        return Ok(AgentTurn {
            user_text,
            reply: String::new(),
            voiced: false,
            from_audio,
            skipped: true,
        });
    }

    // 3. Chat turn.
    let reply = chat.reply(&user_text).await?;

    // 4. Text reply (always).
    sink.reply_text(&msg.room, &reply).await?;

    // 4b. Voice reply (full voice-to-voice outbound leg).
    let mut voiced = false;
    if cfg.voice {
        if let Some(tts) = speaker {
            let (audio, mime) = tts.to_audio(&reply).await?;
            sink.reply_audio(&msg.room, &audio, &mime).await?;
            voiced = true;
        }
    }

    Ok(AgentTurn { user_text, reply, voiced, from_audio, skipped: false })
}

// ── Adapter: MessagingChannel → Sink ─────────────────────────────────────────

/// Bridges any [`MessagingChannel`] onto the object-safe [`Sink`] trait. Audio
/// replies are written to a temp file because the channel `send_file` API is
/// path-based.
pub(crate) struct ChannelSink<C: MessagingChannel> {
    channel: C,
}

impl<C: MessagingChannel> ChannelSink<C> {
    /// Wrap a channel.
    pub(crate) fn new(channel: C) -> Self {
        Self { channel }
    }

    /// Borrow the wrapped channel (for the inbound stream).
    pub(crate) fn channel(&self) -> &C {
        &self.channel
    }
}

#[async_trait]
impl<C: MessagingChannel> Sink for ChannelSink<C> {
    async fn reply_text(&self, room: &str, text: &str) -> Result<(), AgentError> {
        self.channel
            .send_text(room, text)
            .await
            .map(|_| ())
            .map_err(|e| AgentError::Sink(e.to_string()))
    }

    async fn reply_audio(&self, room: &str, audio: &[u8], mime: &str) -> Result<(), AgentError> {
        let ext = match mime {
            "audio/mpeg" => "mp3",
            "audio/wav" | "audio/x-wav" => "wav",
            "audio/ogg" => "ogg",
            _ => "bin",
        };
        let path = std::env::temp_dir().join(format!("aphrody-voice-{}.{ext}", uuid_like()));
        tokio::fs::write(&path, audio)
            .await
            .map_err(|e| AgentError::Sink(format!("temp write: {e}")))?;
        let res = self
            .channel
            .send_file(room, Path::new(&path), mime)
            .await
            .map(|_| ())
            .map_err(|e| AgentError::Sink(e.to_string()));
        let _ = tokio::fs::remove_file(&path).await;
        res
    }
}

/// Cheap, dependency-free unique suffix for temp filenames (nanos + counter).
fn uuid_like() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{n}-{}", CTR.fetch_add(1, Ordering::Relaxed))
}

// ── Adapter: ChatLoop → ChatEngine ────────────────────────────────────────────

/// Bridges the [`aphrody_chat::ChatLoop`] onto the object-safe [`ChatEngine`]
/// trait. The loop's backend (agy → Cloud Code, or the keyless web transport)
/// runs Gemini 3.5 Flash by default.
pub(crate) struct ChatLoopEngine {
    loop_: aphrody_chat::ChatLoop,
}

impl ChatLoopEngine {
    /// Wrap a built chat loop.
    pub(crate) fn new(loop_: aphrody_chat::ChatLoop) -> Self {
        Self { loop_ }
    }
}

#[async_trait]
impl ChatEngine for ChatLoopEngine {
    async fn reply(&mut self, user_text: &str) -> Result<String, AgentError> {
        self.loop_
            .single_turn(user_text)
            .await
            .map(|turn| turn.assistant_response)
            .map_err(|e| AgentError::Chat(e.to_string()))
    }
}

// ── Sink: stdout (headless `--simulate` + dry-run) ────────────────────────────

/// A [`Sink`] that prints replies to stdout — used by `--simulate` so the full
/// pipeline can be exercised headlessly (no live channel credentials), and as a
/// dry-run target.
pub(crate) struct StdoutSink;

#[async_trait]
impl Sink for StdoutSink {
    async fn reply_text(&self, room: &str, text: &str) -> Result<(), AgentError> {
        println!("[{room}] {text}");
        Ok(())
    }
    async fn reply_audio(&self, room: &str, audio: &[u8], mime: &str) -> Result<(), AgentError> {
        println!("[{room}] <audio {} bytes, {mime}>", audio.len());
        Ok(())
    }
}

/// Fetch an audio attachment over HTTP (the production `fetch_audio` injected
/// into [`process_inbound`]).
///
/// # Errors
/// [`AgentError::Fetch`] on transport failure or a non-2xx response.
pub(crate) async fn http_fetch_audio(url: String) -> Result<Vec<u8>, AgentError> {
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| AgentError::Fetch(format!("GET {url}: {e}")))?
        .error_for_status()
        .map_err(|e| AgentError::Fetch(format!("status: {e}")))?;
    resp.bytes().await.map(|b| b.to_vec()).map_err(|e| AgentError::Fetch(format!("body: {e}")))
}

// ── Live voice adapters (aphrody-voice) ───────────────────────────────────────

/// TTS adapter: ElevenLabs → [`Speaker`]. Voice id from `ELEVENLABS_VOICE_ID`
/// or a sensible default.
pub(crate) struct ElevenSpeaker {
    provider: aphrody_voice::elevenlabs::ElevenLabsProvider,
    voice_id: String,
}

impl ElevenSpeaker {
    /// Build from `ELEVENLABS_API_KEY` (+ optional `ELEVENLABS_VOICE_ID`).
    pub(crate) fn from_env() -> Result<Self, AgentError> {
        let key = std::env::var("ELEVENLABS_API_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AgentError::Tts("ELEVENLABS_API_KEY not set".into()))?;
        let voice_id = std::env::var("ELEVENLABS_VOICE_ID")
            .unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_owned()); // ElevenLabs "Rachel"
        Ok(Self { provider: aphrody_voice::elevenlabs::ElevenLabsProvider::new(key), voice_id })
    }
}

#[async_trait]
impl Speaker for ElevenSpeaker {
    async fn to_audio(&self, text: &str) -> Result<(Vec<u8>, String), AgentError> {
        use aphrody_voice::VoiceProvider;
        let bytes = self
            .provider
            .synthesize(text, &self.voice_id, aphrody_voice::SynthOptions::default())
            .await
            .map_err(|e| AgentError::Tts(e.to_string()))?;
        Ok((bytes, "audio/mpeg".to_owned()))
    }
}

/// STT adapter: OpenAI Whisper API → [`Transcriber`].
pub(crate) struct WhisperTranscriber {
    provider: aphrody_voice::stt::whisper_api::WhisperApiProvider,
}

impl WhisperTranscriber {
    /// Build from `OPENAI_API_KEY`.
    pub(crate) fn from_env() -> Result<Self, AgentError> {
        let provider = aphrody_voice::stt::whisper_api::WhisperApiProvider::from_env()
            .map_err(|e| AgentError::Stt(e.to_string()))?;
        Ok(Self { provider })
    }
}

#[async_trait]
impl Transcriber for WhisperTranscriber {
    async fn to_text(&self, audio: &[u8], _mime: &str) -> Result<String, AgentError> {
        use aphrody_voice::stt::SttProvider;
        let t = self
            .provider
            .transcribe(audio, aphrody_voice::stt::SttOptions::default())
            .await
            .map_err(|e| AgentError::Stt(e.to_string()))?;
        Ok(t.text)
    }
}

// ── Channel selection + run loop ──────────────────────────────────────────────

/// Which messaging channel the agent runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HermesChannel {
    /// Discord (REST v10 bot, `DISCORD_BOT_TOKEN`).
    Discord,
    /// X / Twitter (cookie-auth `aphrody-x` binary).
    X,
}

/// Drive the agent loop over a concrete channel: stream inbox → process each.
async fn run_loop<C>(
    channel: C,
    mut chat: ChatLoopEngine,
    transcriber: Option<WhisperTranscriber>,
    speaker: Option<ElevenSpeaker>,
    cfg: AgentConfig,
) -> Result<(), AgentError>
where
    C: MessagingChannel,
{
    use futures::StreamExt as _;

    let sink = ChannelSink::new(channel);
    let stream = sink
        .channel()
        .stream_inbox()
        .await
        .map_err(|e| AgentError::Sink(format!("stream_inbox init: {e}")))?;
    let mut stream = Box::pin(stream);

    eprintln!("[hermes] listening (voice={}, trigger={:?})…", cfg.voice, cfg.trigger);
    while let Some(item) = stream.next().await {
        let msg = match item {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[hermes] inbound error (continuing): {e}");
                continue;
            },
        };
        let t = transcriber.as_ref().map(|x| x as &dyn Transcriber);
        let s = speaker.as_ref().map(|x| x as &dyn Speaker);
        match process_inbound(&msg, &mut chat, t, s, &sink, &cfg, http_fetch_audio).await {
            Ok(turn) if !turn.skipped => {
                eprintln!("[hermes] replied to {} (voiced={})", msg.sender, turn.voiced);
            },
            Ok(_) => {},
            Err(e) => eprintln!("[hermes] turn error (continuing): {e}"),
        }
    }
    Ok(())
}

/// `aphrody hermes` entry point.
///
/// Builds the Gemini-3.5-Flash chat loop, optional voice-to-voice (STT/TTS),
/// then either processes one synthetic message (`simulate`) for headless
/// verification, or streams the chosen live channel.
///
/// # Errors
/// Surfaces [`AgentError`] / channel-construction failures.
pub(crate) async fn run(
    channel: HermesChannel,
    voice: bool,
    trigger: String,
    model: Option<String>,
    stub: bool,
    simulate: Option<String>,
) -> miette::Result<()> {
    use aphrody_chat::{
        ChatConfig, ChatLoop,
        backend::{ModelBackend, StubBackend},
    };

    use crate::agy_backend::AgyBackend;

    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut config = ChatConfig::default();
    if let Some(m) = model {
        config.model = m;
    }

    // Backend: stub (offline CI) → web (keyless Gemini 3.5 Flash) → agy
    // (Cloud Code, default; Gemini flash tier).
    let model_str = config.model.clone();
    let backend: Box<dyn ModelBackend> = if stub {
        Box::new(StubBackend::with_reply(
            "(stub backend reply — `aphrody hermes --stub`, no live LLM call)",
        ))
    } else {
        match AgyBackend::connect(Some(model_str.as_str())) {
            Ok(b) => Box::new(b),
            Err(e) => return Err(miette::miette!("agy backend unavailable: {e}")),
        }
    };

    let chat_loop = ChatLoop::builder(config)
        .with_boxed_backend(backend)
        .build()
        .map_err(|e| miette::miette!("ChatLoop build failed: {e}"))?;
    let mut chat = ChatLoopEngine::new(chat_loop);

    // Optional voice-to-voice providers.
    let (transcriber, speaker) = if voice {
        let t = WhisperTranscriber::from_env()
            .map_err(|e| miette::miette!("STT unavailable for --voice: {e}"))?;
        let s = ElevenSpeaker::from_env()
            .map_err(|e| miette::miette!("TTS unavailable for --voice: {e}"))?;
        (Some(t), Some(s))
    } else {
        (None, None)
    };

    let cfg = AgentConfig { voice, trigger };

    // Headless one-shot: process a synthetic message through the real pipeline.
    if let Some(text) = simulate {
        let msg = InboundMessage {
            id: "simulate".into(),
            room: "simulate".into(),
            sender: "local".into(),
            text,
            attachments: vec![],
            timestamp: chrono::Utc::now(),
        };
        let t = transcriber.as_ref().map(|x| x as &dyn Transcriber);
        let s = speaker.as_ref().map(|x| x as &dyn Speaker);
        let sink = StdoutSink;
        let turn = process_inbound(&msg, &mut chat, t, s, &sink, &cfg, http_fetch_audio)
            .await
            .map_err(|e| miette::miette!("simulate failed: {e}"))?;
        eprintln!("[hermes] simulate done (skipped={}, voiced={})", turn.skipped, turn.voiced);
        return Ok(());
    }

    // Live: build the channel and stream.
    match channel {
        HermesChannel::Discord => {
            let ch = aphrody_messaging::channels::discord::DiscordChannel::from_env()
                .map_err(|e| miette::miette!("Discord channel: {e}"))?;
            run_loop(ch, chat, transcriber, speaker, cfg)
                .await
                .map_err(|e| miette::miette!("hermes loop: {e}"))
        },
        HermesChannel::X => {
            let ch = aphrody_messaging::channels::x::XChannel::from_env()
                .map_err(|e| miette::miette!("X channel: {e}"))?;
            run_loop(ch, chat, transcriber, speaker, cfg)
                .await
                .map_err(|e| miette::miette!("hermes loop: {e}"))
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;

    use super::*;

    /// Stub chat engine: echoes a fixed reply, records the prompt it saw.
    struct EchoChat {
        reply: String,
        seen: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl ChatEngine for EchoChat {
        async fn reply(&mut self, user_text: &str) -> Result<String, AgentError> {
            self.seen.lock().unwrap().push(user_text.to_owned());
            Ok(self.reply.clone())
        }
    }

    struct FixedStt(&'static str);
    #[async_trait]
    impl Transcriber for FixedStt {
        async fn to_text(&self, _audio: &[u8], _mime: &str) -> Result<String, AgentError> {
            Ok(self.0.to_owned())
        }
    }

    struct FixedTts;
    #[async_trait]
    impl Speaker for FixedTts {
        async fn to_audio(&self, text: &str) -> Result<(Vec<u8>, String), AgentError> {
            Ok((format!("AUDIO:{text}").into_bytes(), "audio/mpeg".to_owned()))
        }
    }

    #[derive(Default)]
    struct RecSink {
        texts: Mutex<Vec<(String, String)>>,
        audios: Mutex<Vec<(String, Vec<u8>, String)>>,
    }
    #[async_trait]
    impl Sink for RecSink {
        async fn reply_text(&self, room: &str, text: &str) -> Result<(), AgentError> {
            self.texts.lock().unwrap().push((room.to_owned(), text.to_owned()));
            Ok(())
        }
        async fn reply_audio(
            &self,
            room: &str,
            audio: &[u8],
            mime: &str,
        ) -> Result<(), AgentError> {
            self.audios.lock().unwrap().push((room.to_owned(), audio.to_vec(), mime.to_owned()));
            Ok(())
        }
    }

    fn msg(text: &str, attachments: Vec<Attachment>) -> InboundMessage {
        InboundMessage {
            id: "m1".into(),
            room: "room1".into(),
            sender: "alice".into(),
            text: text.into(),
            attachments,
            timestamp: Utc::now(),
        }
    }

    fn audio_att(mime: &str, url: Option<&str>) -> Attachment {
        Attachment {
            id: "a1".into(),
            filename: "memo.mp3".into(),
            mime_type: mime.into(),
            url: url.map(str::to_owned),
            size_bytes: Some(1024),
        }
    }

    async fn no_fetch(_url: String) -> Result<Vec<u8>, AgentError> {
        Err(AgentError::Fetch("unexpected fetch".into()))
    }

    #[tokio::test]
    async fn text_only_path_replies_with_text() {
        let mut chat = EchoChat { reply: "hi back".into(), seen: Mutex::new(vec![]) };
        let sink = RecSink::default();
        let cfg = AgentConfig::default();
        let turn =
            process_inbound(&msg("hello", vec![]), &mut chat, None, None, &sink, &cfg, no_fetch)
                .await
                .unwrap();
        assert_eq!(turn.user_text, "hello");
        assert_eq!(turn.reply, "hi back");
        assert!(!turn.voiced && !turn.from_audio && !turn.skipped);
        assert_eq!(sink.texts.lock().unwrap().as_slice(), &[("room1".into(), "hi back".into())]);
        assert!(sink.audios.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn voice_to_voice_transcribes_then_synthesizes() {
        let mut chat = EchoChat { reply: "voiced reply".into(), seen: Mutex::new(vec![]) };
        let stt = FixedStt("spoken question");
        let tts = FixedTts;
        let sink = RecSink::default();
        let cfg = AgentConfig { voice: true, trigger: String::new() };
        let fetch = |_u: String| async { Ok(b"rawbytes".to_vec()) };

        let m = msg("", vec![audio_att("audio/mpeg", Some("https://x/memo.mp3"))]);
        let turn = process_inbound(&m, &mut chat, Some(&stt), Some(&tts), &sink, &cfg, fetch)
            .await
            .unwrap();

        // Inbound leg: audio was transcribed and fed to chat.
        assert!(turn.from_audio);
        assert_eq!(chat.seen.lock().unwrap().as_slice(), &["spoken question".to_owned()]);
        // Outbound leg: both text and synthesised audio were delivered.
        assert!(turn.voiced);
        assert_eq!(sink.texts.lock().unwrap().len(), 1);
        let audios = sink.audios.lock().unwrap();
        assert_eq!(audios.len(), 1);
        assert_eq!(audios[0].2, "audio/mpeg");
        assert_eq!(audios[0].1, b"AUDIO:voiced reply");
    }

    #[tokio::test]
    async fn trigger_gate_skips_unmatched() {
        let mut chat = EchoChat { reply: "x".into(), seen: Mutex::new(vec![]) };
        let sink = RecSink::default();
        let cfg = AgentConfig { voice: false, trigger: "@aphrody".into() };
        let turn = process_inbound(
            &msg("just chatter", vec![]),
            &mut chat,
            None,
            None,
            &sink,
            &cfg,
            no_fetch,
        )
        .await
        .unwrap();
        assert!(turn.skipped);
        assert!(chat.seen.lock().unwrap().is_empty());
        assert!(sink.texts.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn empty_text_is_skipped() {
        let mut chat = EchoChat { reply: "x".into(), seen: Mutex::new(vec![]) };
        let sink = RecSink::default();
        let cfg = AgentConfig::default();
        let turn =
            process_inbound(&msg("   ", vec![]), &mut chat, None, None, &sink, &cfg, no_fetch)
                .await
                .unwrap();
        assert!(turn.skipped);
        assert!(sink.texts.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn voice_off_ignores_audio_attachment() {
        let mut chat = EchoChat { reply: "r".into(), seen: Mutex::new(vec![]) };
        let stt = FixedStt("should not be used");
        let sink = RecSink::default();
        let cfg = AgentConfig { voice: false, trigger: String::new() };
        let m = msg("typed text", vec![audio_att("audio/mpeg", Some("https://x/a.mp3"))]);
        let turn =
            process_inbound(&m, &mut chat, Some(&stt), None, &sink, &cfg, no_fetch).await.unwrap();
        assert!(!turn.from_audio);
        assert_eq!(chat.seen.lock().unwrap().as_slice(), &["typed text".to_owned()]);
    }

    #[test]
    fn first_audio_picks_audio_mime_only() {
        let m = msg("", vec![
            Attachment {
                id: "img".into(),
                filename: "p.png".into(),
                mime_type: "image/png".into(),
                url: None,
                size_bytes: None,
            },
            audio_att("audio/ogg", Some("u")),
        ]);
        assert_eq!(first_audio(&m).map(|a| a.id.as_str()), Some("a1"));
    }
}
