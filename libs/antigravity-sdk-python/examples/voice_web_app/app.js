/* Copyright 2026 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// UI Elements
const sidebar = document.getElementById("sidebar");
const menuToggleBtn = document.getElementById("menuToggleBtn");
const wsUrlInput = document.getElementById("wsUrl");
const voiceSelect = document.getElementById("voiceSelect");
const energyThresholdInput = document.getElementById("energyThreshold");
const thresholdVal = document.getElementById("thresholdVal");
const silenceTimeoutInput = document.getElementById("silenceTimeout");
const timeoutVal = document.getElementById("timeoutVal");
const micMonitorFill = document.getElementById("micMonitorFill");
const micMonitorThreshold = document.getElementById("micMonitorThreshold");
const connBadge = document.getElementById("connBadge");
const connBadgeText = document.getElementById("connBadgeText");
const visualizerContainer = document.getElementById("visualizerContainer");
const gemSphere = document.getElementById("gemSphere");
const gemIcon = document.getElementById("gemIcon");
const statusTextDisplay = document.getElementById("statusTextDisplay");
const clearLogBtn = document.getElementById("clearLogBtn");
const chatLog = document.getElementById("chatLog");
const textPromptInput = document.getElementById("textPromptInput");
const sendPromptBtn = document.getElementById("sendPromptBtn");
const muteMicBtn = document.getElementById("muteMicBtn");
const muteIcon = document.getElementById("muteIcon");
const waveformCanvas = document.getElementById("waveformCanvas");
const canvasCtx = waveformCanvas.getContext("2d");

// Application State
let ws = null;
let audioContext = null;
let micStream = null;
let micSource = null;
let micProcessor = null;

// VAD & Streaming state
let isMuted = false;
let isConnected = false;
let userIsSpeaking = false;
let silenceSamplesCount = 0;
let currentAppStatus = "idle"; // "idle", "listening", "thinking", "speaking"

// Audio Playback State (Scheduled PCM)
let nextPlaybackTime = 0;
let activeAudioSources = [];
const KOKORO_SAMPLE_RATE = 24000;
const RECORDING_SAMPLE_RATE = 16000;

// Visualization variables
let animationFrameId = null;
let micLevel = 0;
let speakerLevel = 0;
let visualPhase = 0;

// SVG Icons
const MIC_ON_SVG = `
  <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm5.3-3c0 3-2.54 5.1-5.3 5.1S6.7 14 6.7 11H5c0 3.41 2.72 6.23 6 6.72V21h2v-3.28c3.28-.48 6-3.3 6-6.72h-1.7z"/>
  </svg>
`;

const MIC_OFF_SVG = `
  <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
    <path d="M19 11h-1.7c0 .74-.16 1.43-.43 2.05l1.23 1.23c.56-.98.9-2.09.9-3.28zm-4.02.17c0-.06.02-.11.02-.17V5c0-1.66-1.34-3-3-3S9 3.34 9 5v.18l5.98 5.99zM4.27 3L3 4.27l6.01 6.01V11c0 1.66 1.33 3 2.99 3 .22 0 .44-.03.65-.08l1.79 1.79c-.76.51-1.6.88-2.53.97V21h2v-3.28c.91-.09 1.77-.42 2.52-.92l3.74 3.74L21 20.73 4.27 3zM12 14c-1.66 0-3-1.34-3-3V5c0-1.66 1.34-3 3-3S15 3.34 15 5v6c0 1.66-1.34 3-3 3z"/>
  </svg>
`;

// Responsive menu sidebar toggle
menuToggleBtn.addEventListener("click", () => {
  sidebar.classList.toggle("open");
});

// Update slider numeric values on UI
energyThresholdInput.addEventListener("input", (e) => {
  const val = parseFloat(e.target.value);
  thresholdVal.textContent = val.toFixed(3);
  micMonitorThreshold.style.left = `${val * 1000}%`;
});

silenceTimeoutInput.addEventListener("input", (e) => {
  timeoutVal.textContent = `${e.target.value}s`;
});

clearLogBtn.addEventListener("click", () => {
  chatLog.innerHTML = `<div style="text-align: center; color: var(--md-sys-color-on-surface-variant); font-size: 13px; margin: auto;">Log cleared. Speak to the assistant to start.</div>`;
});

// Canvas Setup & Sizing
function resizeCanvas() {
  const dpr = window.devicePixelRatio || 1;
  const rect = visualizerContainer.getBoundingClientRect();
  waveformCanvas.width = rect.width * dpr;
  waveformCanvas.height = rect.height * dpr;
  canvasCtx.scale(dpr, dpr);
}
window.addEventListener("resize", resizeCanvas);
setTimeout(resizeCanvas, 100);

// Visualizer animation loop
function animateVisualizer() {
  visualPhase += 0.05;
  const dpr = window.devicePixelRatio || 1;
  const width = waveformCanvas.width / dpr;
  const height = waveformCanvas.height / dpr;
  const centerX = width / 2;
  const centerY = height / 2;
  
  canvasCtx.clearRect(0, 0, width, height);

  // Set visual parameters based on current app state
  let baseRadius = 80;
  let waveAmp = 0;
  let waveSpeed = 0.05;
  let strokeColors = [];

  if (currentAppStatus === "listening") {
    // User speaking: Pulse with microphone level
    baseRadius = 80 + micLevel * 50;
    waveAmp = 8 + micLevel * 60;
    waveSpeed = 0.12;
    strokeColors = [
      "rgba(66, 133, 244, 0.6)", // Gemini Blue
      "rgba(191, 242, 141, 0.6)"  // Gemini Green
    ];
  } else if (currentAppStatus === "thinking") {
    // Thinking: Smooth wave shift
    baseRadius = 85;
    waveAmp = 12;
    waveSpeed = 0.08;
    strokeColors = [
      "rgba(145, 104, 192, 0.7)", // Gemini Purple
      "rgba(236, 72, 153, 0.7)",  // Gemini Pink
      "rgba(66, 133, 244, 0.7)"   // Gemini Blue
    ];
  } else if (currentAppStatus === "speaking") {
    // Agent speaking: Pulse with agent audio level
    baseRadius = 80 + speakerLevel * 60;
    waveAmp = 10 + speakerLevel * 80;
    waveSpeed = 0.15;
    strokeColors = [
      "rgba(145, 104, 192, 0.6)", // Gemini Purple
      "rgba(236, 72, 153, 0.6)"   // Gemini Pink
    ];
  } else {
    // Idle state: Gentle float
    baseRadius = 75;
    waveAmp = 3;
    waveSpeed = 0.02;
    strokeColors = ["rgba(103, 80, 164, 0.3)"]; // Primary violet
  }

  // Draw circular waves
  for (let w = 0; w < strokeColors.length; w++) {
    canvasCtx.beginPath();
    canvasCtx.strokeStyle = strokeColors[w];
    canvasCtx.lineWidth = 3 - w * 0.5;
    
    const wavePhase = visualPhase * waveSpeed + (w * Math.PI / 2);
    
    for (let angle = 0; angle <= Math.PI * 2; angle += 0.05) {
      // Create organic wavelike modulation
      const mod = Math.sin(angle * 6 + wavePhase) * Math.cos(angle * 3 - wavePhase);
      const radius = baseRadius + mod * waveAmp;
      
      const x = centerX + Math.cos(angle) * radius;
      const y = centerY + Math.sin(angle) * radius;
      
      if (angle === 0) {
        canvasCtx.moveTo(x, y);
      } else {
        canvasCtx.lineTo(x, y);
      }
    }
    
    canvasCtx.closePath();
    canvasCtx.stroke();
  }

  // Ring rotation & scale sync
  const ringScale = 1 + (currentAppStatus === "listening" ? micLevel * 0.4 : (currentAppStatus === "speaking" ? speakerLevel * 0.4 : 0));
  const ringRotation = visualPhase * 10;
  const ring = document.getElementById("gemPulsingRing");
  if (ring) {
    ring.style.transform = `scale(${ringScale}) rotate(${ringRotation}deg)`;
  }

  animationFrameId = requestAnimationFrame(animateVisualizer);
}

// Update app state and adjust classes / text
function updateAppStatus(status) {
  currentAppStatus = status;
  visualizerContainer.className = `visualizer-container state-${status}`;
  
  // Set UI helper labels
  if (status === "listening") {
    statusTextDisplay.textContent = "Listening...";
    statusTextDisplay.style.color = "var(--gemini-brand-blue)";
  } else if (status === "thinking") {
    statusTextDisplay.textContent = "Thinking...";
    statusTextDisplay.style.color = "var(--gemini-brand-purple)";
  } else if (status === "speaking") {
    statusTextDisplay.textContent = "Agent Speaking...";
    statusTextDisplay.style.color = "var(--gemini-brand-pink)";
  } else {
    statusTextDisplay.textContent = "Voice Active - Speak Now";
    statusTextDisplay.style.color = "var(--md-sys-color-on-surface-variant)";
  }
}

// Append Chat Messages
let latestAgentMessageBubble = null;

function appendChatMessage(role, text, isDelta = false) {
  // Clear empty state message if present
  if (chatLog.children.length === 1 && chatLog.firstChild.nodeType === Node.ELEMENT_NODE && chatLog.firstChild.style.textAlign === "center") {
    chatLog.innerHTML = "";
  }

  if (role === "agent" && isDelta && latestAgentMessageBubble) {
    latestAgentMessageBubble.textContent += text;
    chatLog.scrollTop = chatLog.scrollHeight;
    return;
  }

  const bubble = document.createElement("div");
  bubble.className = `chat-message ${role}`;
  bubble.textContent = text;
  chatLog.appendChild(bubble);
  chatLog.scrollTop = chatLog.scrollHeight;

  if (role === "agent") {
    latestAgentMessageBubble = bubble;
  } else {
    latestAgentMessageBubble = null;
  }
}

// Mute Mic click handler
muteMicBtn.addEventListener("click", () => {
  isMuted = !isMuted;
  muteMicBtn.classList.toggle("muted", isMuted);
  muteIcon.innerHTML = isMuted ? MIC_OFF_SVG : MIC_ON_SVG;
});

// Text Send button triggers (optional fallback typing)
sendPromptBtn.addEventListener("click", sendTextMessage);
textPromptInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    sendTextMessage();
  }
});

function sendTextMessage() {
  const text = textPromptInput.value.trim();
  if (!text || !ws || ws.readyState !== WebSocket.OPEN) return;

  // Interrupt if speaking
  stopAllActiveAudio();

  appendChatMessage("user", text);
  textPromptInput.value = "";

  // Simulate VAD start & end for text prompt to server
  ws.send(JSON.stringify({ type: "speech_start" }));
  ws.send(JSON.stringify({ type: "speech_end" }));
  
  // Directly send mock transcript command if custom
  ws.send(JSON.stringify({ type: "start", voice: voiceSelect.value }));
}

// Stop all scheduled playback nodes (interrupt/barge-in)
function stopAllActiveAudio() {
  activeAudioSources.forEach(source => {
    try {
      source.stop();
    } catch (e) {
      // already stopped or not started
    }
  });
  activeAudioSources = [];
  nextPlaybackTime = 0;
  speakerLevel = 0;
  updateAppStatus("idle");
}

// Initialize Web Audio API for recording and processing
async function initAudio() {
  try {
    // Check if AudioContext needs fallback
    const AudioContextClass = window.AudioContext || window.webkitAudioContext;
    
    // Explicitly set sampleRate to 16000 so the browser handles resampling on capture
    audioContext = new AudioContextClass({ sampleRate: RECORDING_SAMPLE_RATE });
    
    // Resume context if suspended (browser security autoplays)
    if (audioContext.state === "suspended") {
      await audioContext.resume();
    }

    // Capture microphone media stream
    micStream = await navigator.mediaDevices.getUserMedia({
      audio: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true
      }
    });

    micSource = audioContext.createMediaStreamSource(micStream);
    
    // Create ScriptProcessorNode (2048 buffer size, 1 input channel, 1 output channel)
    // Runs at 16000Hz due to parent AudioContext sampleRate configuration
    micProcessor = audioContext.createScriptProcessor(2048, 1, 1);
    
    micProcessor.onaudioprocess = (e) => {
      if (isMuted || !isConnected || ws.readyState !== WebSocket.OPEN) {
        micMonitorFill.style.width = "0%";
        return;
      }

      const inputBuffer = e.inputBuffer.getChannelData(0);
      
      // Calculate RMS level for energy tracking
      let sum = 0;
      for (let i = 0; i < inputBuffer.length; i++) {
        sum += inputBuffer[i] * inputBuffer[i];
      }
      const rms = Math.sqrt(sum / inputBuffer.length);
      
      // Update RMS level visual state
      micLevel = rms;
      micMonitorFill.style.width = `${Math.min(rms * 1000, 100)}%`;

      const threshold = parseFloat(energyThresholdInput.value);
      const silenceTimeout = parseFloat(silenceTimeoutInput.value);
      const samplesPerSecond = RECORDING_SAMPLE_RATE;
      const timeoutSamplesLimit = silenceTimeout * samplesPerSecond;

      if (rms > threshold) {
        if (!userIsSpeaking) {
          userIsSpeaking = true;
          console.log("[VAD] Voice detected, starting transmission.");
          // Interrupt any active agent voice response
          stopAllActiveAudio();
          ws.send(JSON.stringify({ type: "speech_start" }));
        }
        silenceSamplesCount = 0;
        
        // Stream raw float32 PCM samples directly to the websocket
        ws.send(inputBuffer.buffer);
      } else if (userIsSpeaking) {
        // Keep sending audio even under threshold until silence timeout is hit
        ws.send(inputBuffer.buffer);
        silenceSamplesCount += inputBuffer.length;

        if (silenceSamplesCount >= timeoutSamplesLimit) {
          userIsSpeaking = false;
          console.log("[VAD] Silence timeout reached, stopping transmission.");
          ws.send(JSON.stringify({ type: "speech_end" }));
        }
      }
    };

    micSource.connect(micProcessor);
    micProcessor.connect(audioContext.destination);

    console.log("Audio pipeline initialized successfully.");
  } catch (err) {
    console.error("Failed to initialize audio input:", err);
    alert("Microphone access is required for voice-to-voice UI: " + err.message);
    disconnect();
  }
}

// Scheduled play of raw Kokoro PCM chunks
function playPCMBuffer(float32Data) {
  if (!audioContext) return;

  const buffer = audioContext.createBuffer(1, float32Data.length, KOKORO_SAMPLE_RATE);
  buffer.copyToChannel(float32Data, 0);

  const source = audioContext.createBufferSource();
  source.buffer = buffer;
  source.connect(audioContext.destination);

  // Keep track of active audio source for interrupts
  activeAudioSources.push(source);
  
  // Cleanup references on end
  source.onended = () => {
    const idx = activeAudioSources.indexOf(source);
    if (idx !== -1) {
      activeAudioSources.splice(idx, 1);
    }
    if (activeAudioSources.length === 0) {
      speakerLevel = 0;
      if (currentAppStatus === "speaking") {
        updateAppStatus("idle");
      }
    }
  };

  // Schedule playback gaplessly
  const currentTime = audioContext.currentTime;
  const playTime = Math.max(currentTime, nextPlaybackTime);
  
  source.start(playTime);
  
  // Calculate average amplitude of the chunk to drive pulse visual animations
  let sum = 0;
  for (let i = 0; i < float32Data.length; i += 10) { // downsample loop for speed
    sum += Math.abs(float32Data[i]);
  }
  const amp = sum / (float32Data.length / 10);
  
  // Schedule animation callback timing
  setTimeout(() => {
    if (activeAudioSources.includes(source)) {
      speakerLevel = Math.min(amp * 1.5, 1.0);
    }
  }, (playTime - currentTime) * 1000);

  nextPlaybackTime = playTime + buffer.duration;
}

// Websocket connection handlers
function connect() {
  const url = wsUrlInput.value.trim();
  console.log(`Connecting to WebSocket: ${url}`);
  
  connBadge.className = "connection-badge";
  connBadgeText.textContent = "Connecting...";

  ws = new WebSocket(url);
  ws.binaryType = "arraybuffer";

  ws.onopen = async () => {
    console.log("WebSocket connection established.");
    isConnected = true;
    
    connBadge.className = "connection-badge connected";
    connBadgeText.textContent = "Connected";
    
    // Enable controls
    textPromptInput.removeAttribute("disabled");
    sendPromptBtn.removeAttribute("disabled");
    
    // Send configure start frame
    ws.send(JSON.stringify({
      type: "start",
      voice: voiceSelect.value
    }));

    // Initialize audio input and start visualization loops
    await initAudio();
    updateAppStatus("idle");
    animateVisualizer();
  };

  ws.onmessage = (event) => {
    // Raw binary frame is float32 synthesized speaker PCM
    if (event.data instanceof ArrayBuffer) {
      const float32Array = new Float32Array(event.data);
      playPCMBuffer(float32Array);
      return;
    }

    // Text JSON events
    try {
      const msg = json = JSON.parse(event.data);
      
      if (msg.type === "status") {
        updateAppStatus(msg.status);
      } 
      else if (msg.type === "transcript") {
        appendChatMessage(msg.role, msg.text, msg.is_delta);
      } 
      else if (msg.type === "interrupt") {
        console.log("[WS] Server processed interrupt command.");
        stopAllActiveAudio();
      } 
      else if (msg.type === "error") {
        console.error("[WS] Server error:", msg.message);
        appendChatMessage("agent", `[Error: ${msg.message}]`);
      }
    } catch (e) {
      console.warn("Could not parse JSON socket frame:", e);
    }
  };

  ws.onclose = () => {
    console.log("WebSocket connection closed.");
    disconnect();
  };

  ws.onerror = (err) => {
    console.error("WebSocket connection error:", err);
    connBadge.className = "connection-badge error";
    connBadgeText.textContent = "Error";
    disconnect();
  };
}

function disconnect() {
  isConnected = false;
  if (ws) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.close();
    }
    ws = null;
  }

  // Clear Audio context inputs
  if (micProcessor) {
    micProcessor.disconnect();
    micProcessor = null;
  }
  if (micSource) {
    micSource.disconnect();
    micSource = null;
  }
  if (micStream) {
    micStream.getTracks().forEach(track => track.stop());
    micStream = null;
  }
  if (audioContext) {
    audioContext.close();
    audioContext = null;
  }

  stopAllActiveAudio();
  cancelAnimationFrame(animationFrameId);
  animationFrameId = null;

  // Clear canvas
  canvasCtx.clearRect(0, 0, waveformCanvas.width, waveformCanvas.height);
  
  // Reset badge
  connBadge.className = "connection-badge";
  connBadgeText.textContent = "Disconnected";
  
  // Disable UI
  textPromptInput.setAttribute("disabled", "true");
  sendPromptBtn.setAttribute("disabled", "true");
  
  visualizerContainer.className = "visualizer-container state-idle";
  statusTextDisplay.textContent = "Tap the sphere to connect";
  statusTextDisplay.style.color = "var(--md-sys-color-on-surface-variant)";
}

// Toggle connection by clicking Gem sphere
gemSphere.addEventListener("click", () => {
  if (isConnected) {
    disconnect();
  } else {
    connect();
  }
});

// Update voice selections automatically when changed
voiceSelect.addEventListener("change", (e) => {
  if (isConnected && ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({
      type: "start",
      voice: e.target.value
    }));
  }
});
