// SPDX-License-Identifier: Apache-2.0
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{DeviceSinkBuilder, Decoder};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

/// Represents messages sent to the Voice Engine thread.
#[derive(Debug)]
pub enum VoiceCommand {
    /// Start recording audio from the microphone.
    StartRecording,
    /// Stop recording and process the audio.
    StopRecording,
    /// Play a TTS audio payload (e.g., MP3 or WAV).
    PlayAudio(Vec<u8>),
    /// Stop all current audio playback.
    StopAudio,
}

/// Represents events emitted by the Voice Engine back to the async runtime.
#[derive(Debug)]
pub enum VoiceEvent {
    /// Emitted when recording stops, containing the encoded WAV file bytes.
    AudioCaptured(Vec<u8>),
}

pub struct NativeVoiceEngine {
    tx: mpsc::Sender<VoiceCommand>,
}

impl NativeVoiceEngine {
    pub fn new() -> anyhow::Result<(Self, mpsc::Receiver<VoiceEvent>)> {
        let (tx, mut rx) = mpsc::channel::<VoiceCommand>(32);
        let (event_tx, event_rx) = mpsc::channel::<VoiceEvent>(32);

        // Spawn a dedicated thread for audio handling.
        // We use a standard std::thread because CPAL streams and Rodio sinks
        // often do not implement Send/Sync cleanly enough for async Tasks,
        // and audio requires soft-realtime guarantees anyway.
        std::thread::spawn(move || {
            info!("NativeVoiceEngine thread started.");

            // Initialize Playback (Rodio)
            let sink_handle = match DeviceSinkBuilder::open_default_sink() {
                Ok(res) => res,
                Err(e) => {
                    error!("Failed to initialize audio output stream: {}", e);
                    return;
                }
            };
            
            // In rodio 0.22, we connect a Player to the sink's mixer.
            let mut player = rodio::Player::connect_new(&sink_handle.mixer());

            // Initialize Capture (CPAL)
            let host = cpal::default_host();
            let input_device = match host.default_input_device() {
                Some(device) => device,
                None => {
                    warn!("No default input device found for voice capture.");
                    // We don't return here because playback might still be useful.
                    return; 
                }
            };

            let input_config = match input_device.default_input_config() {
                Ok(config) => config,
                Err(e) => {
                    warn!("Failed to get default input config: {}", e);
                    return;
                }
            };
            
            info!("Voice input initialized with config: {:?}", input_config);

            let mut _recording_stream: Option<cpal::Stream> = None;
            let recording_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

            // Blocking loop receiving commands
            while let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    VoiceCommand::PlayAudio(data) => {
                        info!("Received PlayAudio command ({} bytes)", data.len());
                        // Barge-in: Stop current audio
                        player.pause();
                        player = rodio::Player::connect_new(&sink_handle.mixer());
                        
                        let cursor = Cursor::new(data);
                        match Decoder::try_from(cursor) {
                            Ok(decoder) => {
                                sink_handle.mixer().add(decoder);
                                player.play();
                            }
                            Err(e) => {
                                error!("Failed to decode audio data: {:?}", e);
                            }
                        }
                    }
                    VoiceCommand::StopAudio => {
                        info!("Received StopAudio command");
                        player.pause();
                        player = rodio::Player::connect_new(&sink_handle.mixer());
                    }
                    VoiceCommand::StartRecording => {
                        info!("Received StartRecording command");
                        recording_buffer.lock().unwrap().clear();
                        
                        let buffer_clone = recording_buffer.clone();
                        let channels = input_config.channels();
                        let err_fn = |err| error!("An error occurred on the input audio stream: {}", err);

                        let stream = match input_config.sample_format() {
                            cpal::SampleFormat::F32 => input_device.build_input_stream(
                                &input_config.clone().into(),
                                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                    buffer_clone.lock().unwrap().extend_from_slice(data);
                                },
                                err_fn,
                                None,
                            ),
                            cpal::SampleFormat::I16 => input_device.build_input_stream(
                                &input_config.clone().into(),
                                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                    let floats: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                                    buffer_clone.lock().unwrap().extend_from_slice(&floats);
                                },
                                err_fn,
                                None,
                            ),
                            _ => {
                                error!("Unsupported sample format");
                                continue;
                            }
                        };

                        match stream {
                            Ok(s) => {
                                if let Err(e) = s.play() {
                                    error!("Failed to play recording stream: {}", e);
                                } else {
                                    _recording_stream = Some(s);
                                }
                            }
                            Err(e) => error!("Failed to build recording stream: {}", e),
                        }
                    }
                    VoiceCommand::StopRecording => {
                        info!("Received StopRecording command");
                        _recording_stream = None; // Drops the stream, stopping capture.
                        
                        // Encode the captured samples to WAV
                        let samples = recording_buffer.lock().unwrap().clone();
                        if samples.is_empty() {
                            warn!("No audio samples captured.");
                            continue;
                        }

                        let spec = hound::WavSpec {
                            channels: input_config.channels(),
                            sample_rate: input_config.sample_rate(),
                            bits_per_sample: 32,
                            sample_format: hound::SampleFormat::Float,
                        };

                        let mut cursor = Cursor::new(Vec::new());
                        {
                            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
                            for sample in samples {
                                writer.write_sample(sample).unwrap();
                            }
                            writer.finalize().unwrap();
                        }
                        
                        let wav_bytes = cursor.into_inner();
                        info!("Captured {} bytes of WAV audio.", wav_bytes.len());
                        let _ = event_tx.blocking_send(VoiceEvent::AudioCaptured(wav_bytes));
                    }
                }
            }
            
            info!("NativeVoiceEngine thread shutting down.");
        });

        Ok((Self { tx }, event_rx))
    }

    /// Send a command to the voice engine.
    pub async fn send_command(&self, cmd: VoiceCommand) -> anyhow::Result<()> {
        self.tx.send(cmd).await.map_err(|e| anyhow::anyhow!("Voice engine dead: {}", e))
    }
}

