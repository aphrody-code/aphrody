// SPDX-License-Identifier: Apache-2.0
//! Backend STT hors-ligne via `whisper.cpp` (liaisons Rust `whisper-rs`).
//!
//! # Feature gate
//!
//! Ce module est compilé sur toutes les cibles natives.  L'implémentation
//! **réelle** (`whisper_rs::WhisperContext`) n'est active que lorsque la
//! feature Cargo `local-whisper` est activée.  Sans cette feature, chaque
//! méthode publique renvoie [`SttError::NotImplemented`] avec un message
//! descriptif — jamais `unimplemented!()` / `todo!()`.
//!
//! # Activation
//!
//! ```toml
//! aphrody-voice = { path = "…", features = ["local-whisper"] }
//! ```
//!
//! Un fichier de poids GGML Whisper est requis au runtime.  Téléchargement :
//! <https://huggingface.co/ggerganov/whisper.cpp>
//!
//! # Format d'entrée attendu
//!
//! `transcribe` accepte deux formats pour l'octet-slice `audio` :
//!
//! 1. **WAV PCM 16 kHz mono 16-bit** (format standard de capture micro) —
//!    l'en-tête est parsé, les échantillons i16 sont rééchantillonnés en f32
//!    via `whisper_rs::convert_integer_to_float_audio`.
//! 2. **PCM f32 brut** — si les 4 premiers octets ne correspondent pas à la
//!    signature RIFF, les octets sont réinterprétés comme des f32 LE.
//!
//! # Exigences en ressources
//!
//! | Taille modèle | VRAM / RAM | WER (en) |
//! |---------------|------------|----------|
//! | tiny.en       | ~39 MB     | ~5.7 %   |
//! | base.en       | ~74 MB     | ~4.2 %   |
//! | small.en      | ~244 MB    | ~3.4 %   |
//! | medium.en     | ~769 MB    | ~3.0 %   |
//! | large-v3      | ~1550 MB   | ~2.7 %   |

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

use crate::stt::{SttError, SttOptions, SttProvider, Transcript, TranscriptDelta};
#[cfg(feature = "local-whisper")]
use crate::stt::TranscriptSegment;

// ── LocalWhisperBackend ───────────────────────────────────────────────────────

/// Backend STT hors-ligne utilisant `whisper.cpp` via [`whisper_rs`].
///
/// Lorsque la feature `local-whisper` est désactivée, toutes les méthodes
/// renvoient [`SttError::NotImplemented`] — aucun `unimplemented!()` caché.
///
/// # Exemple
///
/// ```rust,no_run
/// # use aphrody_voice::stt::local_whisper::LocalWhisperBackend;
/// let backend = LocalWhisperBackend::new("/path/to/ggml-base.en.bin");
/// ```
#[derive(Debug, Clone)]
pub struct LocalWhisperBackend {
    /// Chemin vers le fichier de poids GGML.
    pub model_path: String,
}

impl LocalWhisperBackend {
    /// Crée un backend qui chargera `model_path` au premier appel.
    ///
    /// La construction est infaillible et ne touche pas le système de
    /// fichiers ; toute erreur I/O est différée au premier appel de
    /// transcription.
    pub fn new(model_path: impl Into<String>) -> Self {
        Self { model_path: model_path.into() }
    }
}

// ── Implémentation réelle (feature `local-whisper` activée) ──────────────────

#[cfg(feature = "local-whisper")]
mod real_impl {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    use super::*;
    use crate::stt::{SttOptions, TranscriptFormat};

    // ── Conversion WAV → f32 PCM ──────────────────────────────────────────────

    /// Erreur de parsing WAV produite par [`parse_wav_to_f32`].
    #[derive(Debug)]
    pub(super) struct WavError(pub String);

    impl std::fmt::Display for WavError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "WAV parse error: {}", self.0)
        }
    }

    /// Parse un buffer WAV PCM 16 kHz mono 16-bit et retourne les échantillons
    /// en f32 normalisés dans `[-1.0, 1.0]`.
    ///
    /// L'en-tête RIFF/WAVE est validé de manière minimale :
    /// - signature `RIFF` + `WAVE` + `fmt `
    /// - codec PCM (audioFormat == 1)
    ///
    /// Les fichiers WAV avec des chunks supplémentaires (`LIST`, `INFO`, etc.)
    /// avant le chunk `data` sont supportés.
    pub(super) fn parse_wav_to_f32(audio: &[u8]) -> Result<Vec<f32>, WavError> {
        // Longueur minimale : 44 octets (en-tête WAV canonique).
        if audio.len() < 44 {
            return Err(WavError("buffer trop court pour être un WAV valide".into()));
        }

        // Validation signature RIFF
        if &audio[0..4] != b"RIFF" {
            return Err(WavError("signature RIFF absente".into()));
        }
        if &audio[8..12] != b"WAVE" {
            return Err(WavError("signature WAVE absente".into()));
        }

        // Parcours des chunks pour trouver fmt  et data.
        let mut pos = 12usize;
        let mut audio_format: u16 = 0;
        let mut num_channels: u16 = 0;
        let mut bits_per_sample: u16 = 0;
        let mut data_start: usize = 0;
        let mut data_len: usize = 0;

        while pos + 8 <= audio.len() {
            let chunk_id = &audio[pos..pos + 4];
            let chunk_size =
                u32::from_le_bytes([audio[pos + 4], audio[pos + 5], audio[pos + 6], audio[pos + 7]])
                    as usize;
            pos += 8;

            if chunk_id == b"fmt " {
                if chunk_size < 16 {
                    return Err(WavError("chunk fmt  trop court".into()));
                }
                if pos + 16 > audio.len() {
                    return Err(WavError("buffer tronqué dans chunk fmt ".into()));
                }
                audio_format =
                    u16::from_le_bytes([audio[pos], audio[pos + 1]]);
                num_channels =
                    u16::from_le_bytes([audio[pos + 2], audio[pos + 3]]);
                // sample_rate à pos+4 — lu mais non contrôlé ici
                bits_per_sample =
                    u16::from_le_bytes([audio[pos + 14], audio[pos + 15]]);
            } else if chunk_id == b"data" {
                data_start = pos;
                data_len = chunk_size.min(audio.len() - pos);
                // On a tout ce qu'il faut — arrêt.
                break;
            }

            // Alignement pair requis par la spec RIFF.
            let advance = chunk_size + (chunk_size & 1);
            if advance == 0 {
                break;
            }
            pos += advance;
        }

        if data_start == 0 {
            return Err(WavError("chunk data introuvable".into()));
        }

        // Codec PCM (1) uniquement.
        if audio_format != 1 {
            return Err(WavError(format!(
                "audioFormat non supporté : {} (seul PCM=1 est accepté)",
                audio_format
            )));
        }

        let pcm_bytes = &audio[data_start..data_start + data_len];

        match bits_per_sample {
            16 => {
                // i16 LE → f32 normalisé
                if pcm_bytes.len() % 2 != 0 {
                    return Err(WavError("nombre impair d'octets dans chunk data (16-bit)".into()));
                }
                let n_samples_per_ch = pcm_bytes.len() / 2 / (num_channels as usize).max(1);
                let total_i16 = pcm_bytes.len() / 2;
                let mut i16_samples: Vec<i16> = Vec::with_capacity(total_i16);
                for chunk in pcm_bytes.chunks_exact(2) {
                    i16_samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                }

                // Mixdown stéréo → mono si nécessaire.
                let mono_i16: Vec<i16> = if num_channels == 2 {
                    i16_samples
                        .chunks_exact(2)
                        .map(|s| (((s[0] as i32 + s[1] as i32) / 2) as i16))
                        .collect()
                } else {
                    i16_samples
                };

                let _ = n_samples_per_ch; // variable calculée pour future validation
                let mut f32_samples = vec![0.0f32; mono_i16.len()];
                whisper_rs::convert_integer_to_float_audio(&mono_i16, &mut f32_samples)
                    .map_err(|e| WavError(format!("conversion i16→f32 : {e}")))?;
                Ok(f32_samples)
            }
            32 => {
                // f32 LE directement
                if pcm_bytes.len() % 4 != 0 {
                    return Err(WavError("nombre d'octets non multiple de 4 dans chunk data (32-bit)".into()));
                }
                let samples: Vec<f32> = pcm_bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();

                // Mixdown stéréo si nécessaire.
                if num_channels == 2 {
                    Ok(samples.chunks_exact(2).map(|s| (s[0] + s[1]) / 2.0).collect())
                } else {
                    Ok(samples)
                }
            }
            bps => Err(WavError(format!(
                "bits_per_sample non supporté : {} (accepté : 16 ou 32)",
                bps
            ))),
        }
    }

    /// Tente d'interpréter `audio` comme du f32 LE brut (chemin de dernier
    /// recours quand la signature WAV est absente).
    fn as_raw_f32(audio: &[u8]) -> Option<Vec<f32>> {
        if audio.len() % 4 != 0 || audio.is_empty() {
            return None;
        }
        Some(
            audio
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        )
    }

    // ── Transcription synchrone (CPU-bound, tourne sur spawn_blocking) ────────

    /// Exécute la transcription complète dans un contexte synchrone.
    ///
    /// Doit être appelé via `tokio::task::spawn_blocking` depuis le chemin
    /// async pour ne pas bloquer le runtime Tokio.
    pub(super) fn run_transcribe_sync(
        model_path: &str,
        samples: &[f32],
        opts: &SttOptions,
    ) -> Result<Transcript, SttError> {
        // Chargement du modèle (coûteux la première fois — pas de cache ici
        // car LocalWhisperBackend est Clone, non Arc).
        let ctx_params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(model_path, ctx_params)
            .map_err(|e| SttError::Other(format!("whisper init error: {e}")))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| SttError::Other(format!("whisper create_state error: {e}")))?;

        // Paramètres de transcription.
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_temperature(opts.temperature);

        // Langue : None = auto-détection, sinon BCP-47 tag.
        let lang_owned: Option<String> = opts.language.clone();
        if let Some(ref lang) = lang_owned {
            params.set_language(Some(lang.as_str()));
        } else {
            params.set_detect_language(true);
        }

        // Exécution du pipeline complet : PCM → mel spectrogram → encoder →
        // decoder → texte.  `full` prend `&[f32]` à 16 kHz mono.
        state
            .full(params, samples)
            .map_err(|e| SttError::Other(format!("whisper full error: {e}")))?;

        // Collecte des segments.
        let n_segments = state
            .full_n_segments()
            .map_err(|e| SttError::Other(format!("whisper full_n_segments error: {e}")))?;

        let mut full_text = String::new();
        let mut segments_out: Vec<TranscriptSegment> = Vec::with_capacity(n_segments as usize);
        let mut last_end_s: f32 = 0.0;

        for i in 0..n_segments {
            let seg_text = state
                .full_get_segment_text_lossy(i)
                .map_err(|e| SttError::Other(format!("whisper segment text error (i={i}): {e}")))?;

            // Les timestamps Whisper sont en centièmes de seconde (whisper_token_data.t0/t1).
            let t0_cs = state
                .full_get_segment_t0(i)
                .unwrap_or(0);
            let t1_cs = state
                .full_get_segment_t1(i)
                .unwrap_or(0);
            let start_s = t0_cs as f32 / 100.0;
            let end_s   = t1_cs as f32 / 100.0;
            last_end_s = end_s;

            full_text.push_str(&seg_text);
            segments_out.push(TranscriptSegment {
                id:    i as u32,
                start: start_s,
                end:   end_s,
                text:  seg_text,
            });
        }

        // Langue détectée — dépend de detect_language ; on ignore les erreurs
        // car certains modèles (monolingues) ne retournent pas d'ID de langue.
        let detected_lang: Option<String> = state
            .full_lang_id_from_state()
            .ok()
            .and_then(|id| whisper_rs::get_lang_str(id))
            .map(str::to_owned);

        Ok(Transcript {
            text: full_text,
            language: detected_lang.or_else(|| opts.language.clone()),
            duration_s: if last_end_s > 0.0 { Some(last_end_s) } else { None },
            segments: segments_out,
        })
    }
}

// ── SttProvider impl — feature `local-whisper` ACTIVÉE ───────────────────────

#[cfg(feature = "local-whisper")]
#[async_trait]
impl SttProvider for LocalWhisperBackend {
    /// Transcrit `audio` localement via `whisper.cpp`.
    ///
    /// # Formats acceptés
    ///
    /// - WAV PCM 16 kHz mono 16-bit (détection automatique de l'en-tête RIFF)
    /// - WAV PCM 16 kHz mono 32-bit float
    /// - PCM f32 LE brut (fallback si signature RIFF absente)
    ///
    /// # Erreurs
    ///
    /// - [`SttError::EmptyInput`] — `audio` est vide.
    /// - [`SttError::Other`] — échec de chargement du modèle, d'encodage, ou
    ///   de parsing du format audio.
    async fn transcribe(&self, audio: &[u8], opts: SttOptions) -> Result<Transcript, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyInput);
        }

        // Extraction des échantillons f32.
        // Chemin 1 : WAV avec en-tête RIFF.
        // Chemin 2 : f32 LE brut (fallback).
        let samples: Vec<f32> = if audio.len() >= 4 && &audio[..4] == b"RIFF" {
            real_impl::parse_wav_to_f32(audio)
                .map_err(|e| SttError::Other(e.to_string()))?
        } else {
            real_impl::as_raw_f32(audio)
                .ok_or_else(|| SttError::Other(
                    "format audio non reconnu : ni WAV ni f32 LE brut".into(),
                ))?
        };

        // La transcription Whisper est CPU-bound et synchrone.
        // On la délègue à spawn_blocking pour ne pas bloquer le worker Tokio.
        let model_path = self.model_path.clone();
        tokio::task::spawn_blocking(move || {
            real_impl::run_transcribe_sync(&model_path, &samples, &opts)
        })
        .await
        .map_err(|e| SttError::Other(format!("spawn_blocking join error: {e}")))?
    }

    /// Accumule le flux d'entrée, transcrit en une seule passe et émet un
    /// unique [`TranscriptDelta`] final.
    ///
    /// `whisper.cpp` ne dispose pas d'un mode de streaming réel côté API
    /// (le streaming token-by-token requiert un callback interne non exposé
    /// par `whisper-rs`).  Les chunks sont bufférisés et la transcription
    /// est lancée à la fin du flux.
    async fn transcribe_stream(
        &self,
        audio: impl Stream<Item = Bytes> + Send + 'static,
        opts: SttOptions,
    ) -> Result<impl Stream<Item = Result<TranscriptDelta, SttError>> + Send, SttError> {
        use futures::StreamExt as _;

        let mut buf: Vec<u8> = Vec::new();
        futures::pin_mut!(audio);
        while let Some(chunk) = audio.next().await {
            buf.extend_from_slice(&chunk);
        }

        let transcript = self.transcribe(&buf, opts).await?;
        let delta = TranscriptDelta { text: transcript.text, is_final: true, segment_id: 0 };
        Ok(futures::stream::once(async move { Ok(delta) }))
    }
}

// ── SttProvider impl — feature `local-whisper` DÉSACTIVÉE ────────────────────

#[cfg(not(feature = "local-whisper"))]
#[async_trait]
impl SttProvider for LocalWhisperBackend {
    /// Retourne [`SttError::NotImplemented`].
    ///
    /// Activez la feature `local-whisper` et fournissez un fichier de poids
    /// GGML pour obtenir une transcription réelle hors-ligne.
    async fn transcribe(&self, _audio: &[u8], _opts: SttOptions) -> Result<Transcript, SttError> {
        Err(SttError::NotImplemented {
            reason: "LocalWhisperBackend requiert la feature `local-whisper` \
                     et un fichier de poids GGML — voir la doc du module local_whisper",
        })
    }

    /// Retourne [`SttError::NotImplemented`].
    ///
    /// Activez la feature `local-whisper` pour utiliser ce backend.
    async fn transcribe_stream(
        &self,
        _audio: impl Stream<Item = Bytes> + Send + 'static,
        _opts: SttOptions,
    ) -> Result<impl Stream<Item = Result<TranscriptDelta, SttError>> + Send, SttError> {
        Err::<futures::stream::Empty<Result<TranscriptDelta, SttError>>, _>(
            SttError::NotImplemented {
                reason: "LocalWhisperBackend::transcribe_stream requiert la feature `local-whisper` \
                         — voir la doc du module local_whisper",
            },
        )
    }
}

// ── Tests unitaires ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructeur_stocke_le_chemin() {
        let backend = LocalWhisperBackend::new("/models/ggml-base.en.bin");
        assert_eq!(backend.model_path, "/models/ggml-base.en.bin");
    }

    /// Sans la feature `local-whisper`, `transcribe` doit renvoyer
    /// `SttError::NotImplemented` — jamais `unimplemented!()`.
    #[cfg(not(feature = "local-whisper"))]
    #[tokio::test]
    async fn transcribe_renvoie_not_implemented_sans_feature() {
        let backend = LocalWhisperBackend::new("/dev/null");
        let err = backend
            .transcribe(b"fake audio", SttOptions::default())
            .await
            .expect_err("doit renvoyer NotImplemented");
        assert!(
            matches!(err, SttError::NotImplemented { .. }),
            "variante inattendue : {err}"
        );
    }

    #[cfg(not(feature = "local-whisper"))]
    #[tokio::test]
    async fn transcribe_stream_renvoie_not_implemented_sans_feature() {
        let backend = LocalWhisperBackend::new("/dev/null");
        let audio_stream = futures::stream::empty::<Bytes>();
        let result = backend.transcribe_stream(audio_stream, SttOptions::default()).await;
        match result {
            Err(SttError::NotImplemented { .. }) => {},
            Err(other) => panic!("variante inattendue : {other}"),
            Ok(_) => panic!("attendait Err de transcribe_stream, reçu Ok"),
        }
    }

    /// Test du parser WAV interne (uniquement quand la feature est activée).
    #[cfg(feature = "local-whisper")]
    #[test]
    fn parse_wav_valide_16bit() {
        // WAV minimal 16 kHz mono 16-bit, 4 échantillons à 0.
        let mut wav: Vec<u8> = Vec::new();
        // RIFF header
        wav.extend_from_slice(b"RIFF");
        let data_size: u32 = 8u32; // 4 échantillons × 2 octets
        let file_size: u32 = 36 + data_size;
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        // fmt  chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes());  // PCM
        wav.extend_from_slice(&1u16.to_le_bytes());  // mono
        wav.extend_from_slice(&16000u32.to_le_bytes()); // sample_rate
        wav.extend_from_slice(&32000u32.to_le_bytes()); // byte_rate
        wav.extend_from_slice(&2u16.to_le_bytes());  // block_align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits_per_sample
        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        for _ in 0..4u32 {
            wav.extend_from_slice(&0i16.to_le_bytes());
        }

        let result = real_impl::parse_wav_to_f32(&wav);
        assert!(result.is_ok(), "parse WAV 16-bit échoué : {:?}", result.err());
        let samples = result.unwrap();
        assert_eq!(samples.len(), 4);
        assert!(samples.iter().all(|&s| s == 0.0));
    }

    /// WAV avec signature RIFF absente → erreur.
    #[cfg(feature = "local-whisper")]
    #[test]
    fn parse_wav_signature_incorrecte() {
        let bad = b"XXXX\x00\x00\x00\x00WAVE";
        let result = real_impl::parse_wav_to_f32(bad);
        assert!(result.is_err());
    }
}
