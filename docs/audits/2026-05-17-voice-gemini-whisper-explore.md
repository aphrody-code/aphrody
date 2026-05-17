<!-- SPDX-License-Identifier: Apache-2.0 -->

# Voice Surface Audit: Gemini CLI & Whisper
## 2026-05-17

Comparative audit of voice/audio capabilities in google-gemini/gemini-cli (TypeScript)
and openai/whisper (Python), mapped against aphrody voice integration.

## Part A: Gemini CLI Voice Surface

### Overview
Gemini CLI ships production-ready voice mode (STT+TTS) with two backends:
Gemini Live (cloud) and local Whisper (via whisper.cpp).
Implementation: packages/core/src/voice/ with UI binding in packages/cli/src/ui/.

### Files Inspected (9 TypeScript files, ~850 LOC)

1. **audioRecorder.ts (116 l.)** - SoX rec command (16 kHz, 16-bit, mono PCM)
2. **transcriptionProvider.ts (34 l.)** - Abstract STT interface
3. **geminiLiveTranscriptionProvider.ts (178 l.)** - WebSocket to Gemini 3.1 Flash Live
4. **whisperTranscriptionProvider.ts (199 l.)** - whisper-stream CLI with VAD (step=0)
5. **whisperModelManager.ts (107 l.)** - Download ggml models from Hugging Face
6. **transcriptionFactory.ts (41 l.)** - Routes backend selection
7. **responseFormatter.ts (185 l.)** - ANSI/markdown stripping for TTS
8. **useVoiceMode.ts (430 l.)** - React hook: PTT, toggle, multi-turn, grace drain
9. **voice-mode.test.ts (77 l.)** - Model download and initialization tests

---

## Part B: OpenAI Whisper Architecture (Python)

### Core Modules (14 Python files, ~2000 LOC)

**whisper/audio.py (158 l.)**
Audio hyperparameters: SAMPLE_RATE=16000, N_FFT=400, HOP_LENGTH=160, CHUNK_LENGTH=30s,
N_SAMPLES=480000, N_FRAMES=3000, N_MELS=80 or 128.
Functions: load_audio() returns float32 via ffmpeg, log_mel_spectrogram() computes STFT
plus Mel filterbank plus log scaling.

**whisper/model.py** - Encoder/decoder Transformer with ModelDimensions dataclass
**whisper/tokenizer.py** - 99 languages via BCP-47, built on tiktoken
**whisper/__init__.py** - available_models(), load_model() from OpenAI CDN

Model sizes: tiny (39M, 1GB), base (74M, 1GB), small (244M, 2GB), medium (769M, 5GB),
large (1550M, 10GB), turbo (809M, 6GB). Speed: 10x to 1x relative to large.

**whisper/decoding.py** - DecodingOptions: task, language, temperature, beam_size, best_of,
patience, length_penalty. detect_language() via cross-attention probing.

**whisper/timing.py** - DTW alignment for word-level timestamps. backtrace() numba-compiled.

**whisper/transcribe.py** - transcribe(model, audio) slides 30s window, concatenates results.
Returns: text, language, duration, segments.

Licensing: MIT.

---

## Part C: Integration Gaps

### Gap 1: Log-Mel Spectrogram Preprocessing (HIGH)
Status: Not implemented in aphrody.
Upstream: whisper/audio.py:110-157
Impact: Local Whisper stub returns NotImplemented.
Fix: Port preprocessing to Rust via ndarray and FFT, or pyo3 wrapper.
File: aphrody-voice-stt/src/local_whisper.rs

### Gap 2: Word-Level Timestamps (MEDIUM)
Status: aphrody supports TranscriptSegment but NOT DTW alignment.
Upstream: whisper/timing.py:57-80, whisper/decoding.py:100+
Impact: Enables karaoke-style highlighting, per-word confidence.
Fix: Extend TranscriptSegment with words: Vec<WordTiming>.
Files: aphrody-voice-stt/src/lib.rs + whisper_api.rs

### Gap 3: Beam Search and Language Detection (MEDIUM)
Status: aphrody supports temperature only.
Upstream: whisper/decoding.py:80-100, :18-77
Impact: Unlocks higher accuracy, task selection, per-language confidence.
Fix: Add beam_size, best_of, patience, task, length_penalty to SttOptions.
Files: aphrody-voice-stt/src/lib.rs + whisper_api.rs

### Gap 4: Offline Streaming and VAD (LOW)
Status: Gemini CLI ships whisper-stream with VAD. Aphrody has no local streaming.
Upstream: gemini-cli/voice/whisperTranscriptionProvider.ts:77-88 (whisper-stream)
Impact: Live transcription without 30s latency.
Fix: Integrate whisper-stream as feature-gated backend, or wrap whisper-rs.
File: aphrody-voice-stt/src/local_whisper.rs (deferred to post-MVP)

---

## Summary

Files Inspected:
- Gemini CLI: 9 TypeScript, ~850 LOC
- Whisper: 14 Python, ~2000 LOC
- Aphrody: 3 Rust crates, ~1000 LOC total

Total: 32 files, ~3850 LOC.

Top 3 Gaps:
1. Log-Mel preprocessing (HIGH) - upstream: whisper/audio.py:110-157
2. Word-level timestamps (MEDIUM) - upstream: whisper/timing.py, decoding.py
3. Beam search and language detect (MEDIUM) - upstream: whisper/decoding.py:18-100

