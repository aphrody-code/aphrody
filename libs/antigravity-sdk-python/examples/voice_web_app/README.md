# Material Design 3 Voice-to-Voice Web Application

This is a local, real-time speech-to-speech AI voice assistant web application that runs entirely offline without requiring external API keys. It uses the `google-antigravity` Python SDK with:
- **Speech-to-Text (STT):** Local CTranslate2-based Whisper model (`faster-whisper`).
- **Text-to-Speech (TTS):** Local ONNX-based Kokoro model (`kokoro-onnx`).
- **Orchestration:** Standard Antigravity `Agent` with `LocalConnectionStrategy`.
- **UI:** A pixel-perfect Google Material Design 3 and Gemini brand-inspired web interface, using Vanilla CSS, a pulsing visualizer gem, and local Voice Activity Detection (VAD) in the browser.

---

## Architecture

```
                       [ Browser Web Page ]
                                |
             (WebSocket Connection ws://localhost:8789)
                                |
                                v
                      [ server.py (Backend) ]
         _______________________|_______________________
        |                       |                       |
        v                       v                       v
 [ Local Whisper STT ]    [ Antigravity Agent ]   [ Local Kokoro TTS ]
```

1. **Microphone Capture:** The browser captures mic input at 16kHz mono.
2. **Client-Side VAD:** Real-time RMS monitoring in JavaScript detects speech boundaries.
3. **Audio Streaming:** Binary float32 PCM frames are streamed over WebSockets while user speaks.
4. **Whisper Transcription:** On silence timeout, the server transcribes speech using Whisper.
5. **Agent Execution:** The transcript is sent to the local `Agent` which yields a text response stream.
6. **Concurrent Synthesis:** Text chunks are sent to Kokoro TTS on punctuation boundaries.
7. **Gapless Audio Playback:** Synthesized 24kHz float32 PCM frames are streamed back to the client and scheduled using AudioContext.
8. **Barge-in Interrupt:** User speaking during agent output triggers immediate cancellation of the active response.

---

## Setup & Installation

### 1. Set Up Python Environment

Ensure you have installed the `voice` dependencies extra of the `google-antigravity` package.

Using `uv`:
```bash
# In the libs/antigravity-sdk-python directory
uv pip install -e ".[voice]"
uv pip install websockets
```

### 2. Run the WebSocket Server

Start the server using `server.py`:
```bash
python server.py
```

*Note: The server will automatically download the default Kokoro ONNX model (`kokoro-v0_19.onnx`, ~80MB) and voices config (`voices.json`, ~20MB) from Hugging Face into a local `./models/` directory on first launch.*

You can also customize parameters via command line:
```bash
python server.py --host 127.0.0.1 --port 8789 --whisper-model small --voice-name af_bella
```

---

## Running the Web Frontend

Since the frontend is built using standard HTML5 APIs and Vanilla CSS, you can open it directly in your browser:

1. Double-click [index.html](index.html) or run a simple local web server:
   ```bash
   # Using Python
   python -m http.server 8000
   # Using Bun
   bunx serve
   ```
2. Open `http://localhost:8000` (or the local file path) in Chrome, Edge, or Firefox.
3. Click the glowing visualizer Gem in the center of the screen to connect to the backend WebSocket.
4. Allow microphone access when prompted.
5. Speak! The visualizer will scale and pulse, and the conversation log will display transcripts in real time.

---

## Design System Integration

The design implements the specifications from the parent repository [DESIGN.md](../../../DESIGN.md):
- **Typography:** Uses Outfit / Google Sans Flex for variable typographic weight.
- **Color Palette:** Strictly adheres to M3 baseline values (violet primary, container shades) and the Gemini brand spectrum gradient (blue → purple → pink).
- **Layout:** Displays a modern sidebar for threshold configuration, a centered visual arena for speech cues, and a collapsible chat log for history tracking.
