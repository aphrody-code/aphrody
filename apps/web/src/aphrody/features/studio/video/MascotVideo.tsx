// SPDX-License-Identifier: Apache-2.0
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";

export interface MascotVideoProps {
  template: string;
  title: string;
  subtitle: string;
  theme: string;
  speechText: string;
}

export function MascotVideo({ template, title, subtitle, theme, speechText }: MascotVideoProps) {
  const frame = useCurrentFrame();
  const config = useVideoConfig();

  // 1. Animate background gradient angle based on time
  const bgAngle = interpolate(frame, [0, config.durationInFrames], [135, 495]);
  const currentThemeGradient = (() => {
    switch (theme) {
      case "cyber":
        return `linear-gradient(${bgAngle}deg, #f12711 0%, #f5af19 100%)`;
      case "mint":
        return `linear-gradient(${bgAngle}deg, #11998e 0%, #38ef7d 100%)`;
      case "midnight":
        return `linear-gradient(${bgAngle}deg, #0f2027 0%, #203a43 50%, #2c5364 100%)`;
      default: // sparkle
        return `linear-gradient(${bgAngle}deg, var(--md-sys-color-primary) 0%, var(--md-sys-color-tertiary) 100%)`;
    }
  })();

  // 2. Spring entry animation for the Mascot / Visual elements
  const mascotScale = spring({
    frame,
    fps: config.fps,
    config: { damping: 11, stiffness: 80, mass: 0.8 },
  });

  // Calculate rotation index (0 to 7) based on timeline to spin the mascot
  const totalRotations = 3; // Number of full spins over duration
  const frameIndex = Math.floor(
    interpolate(
      frame,
      [0, config.durationInFrames],
      [0, totalRotations * 8],
      { extrapolateRight: "clamp" }
    )
  ) % 8;
  const currentMascotUrl = `/assets/aphrody_body_r${frameIndex}.webp`;

  // 3. Slide and fade typography transitions
  const titleOpacity = interpolate(frame, [0, 20], [0, 1], {
    extrapolateRight: "clamp",
    extrapolateLeft: "clamp",
  });
  const titleTranslateY = interpolate(frame, [0, 20], [30, 0], {
    extrapolateRight: "clamp",
    extrapolateLeft: "clamp",
  });

  const subtitleOpacity = interpolate(frame, [10, 30], [0, 1], {
    extrapolateRight: "clamp",
    extrapolateLeft: "clamp",
  });

  // 4. Captions: Word-by-word highlights
  const words = speechText.split(" ");
  const framesPerWord = config.durationInFrames / Math.max(1, words.length);

  // 5. Procedural audio waveform bars
  const waveBars = template === "presentation" ? 24 : 16;

  return (
    <AbsoluteFill
      style={{
        background: currentThemeGradient,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        color: "#fff",
        fontFamily: "Outfit, system-ui, sans-serif",
        boxSizing: "border-box",
        padding: 40,
      }}
    >
      {/* Dynamic Header */}
      <div
        style={{
          position: "absolute",
          top: 40,
          left: 40,
          opacity: titleOpacity,
          transform: `translateY(${titleTranslateY}px)`,
          textShadow: "0 4px 12px rgba(0,0,0,0.35)",
        }}
      >
        <h1 style={{ margin: 0, fontSize: 36, fontWeight: 700, letterSpacing: "-0.5px" }}>
          {title}
        </h1>
        <p style={{ margin: "6px 0 0", fontSize: 16, opacity: subtitleOpacity }}>
          {subtitle}
        </p>
      </div>

      {/* Top Right Logo Watermark */}
      <span
        style={{
          position: "absolute",
          top: 40,
          right: 40,
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
          fontSize: 14,
          fontWeight: "bold",
          color: "rgba(255,255,255,0.7)",
          background: "rgba(0,0,0,0.3)",
          padding: "8px 16px",
          borderRadius: 20,
          backdropFilter: "blur(8px)",
          border: "1px solid rgba(255,255,255,0.1)",
        }}
      >
        ★ aphrody studio
      </span>

      {/* Mascot character with spring entry and rotating frames */}
      {template === "mascot" && (
        <div
          style={{
            transform: `scale(${mascotScale})`,
            height: "50%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <img
            src={currentMascotUrl}
            alt="Mascot"
            style={{
              height: "100%",
              objectFit: "contain",
              filter: "drop-shadow(0 15px 30px rgba(0,0,0,0.45))",
            }}
          />
        </div>
      )}

      {/* Showcase Terminal View */}
      {template === "showcase" && (
        <div
          style={{
            width: "80%",
            height: "45%",
            background: "rgba(10, 10, 12, 0.85)",
            backdropFilter: "blur(20px)",
            borderRadius: 24,
            border: "1px solid rgba(255, 255, 255, 0.15)",
            padding: 24,
            boxShadow: "0 20px 50px rgba(0,0,0,0.5)",
            fontSize: 16,
            fontFamily: "monospace",
            color: "#38ef7d",
            textAlign: "left",
            overflow: "hidden",
            boxSizing: "border-box",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <div style={{ display: "flex", gap: 8, marginBottom: 16, borderBottom: "1px solid rgba(255,255,255,0.1)", paddingBottom: 12 }}>
            <div style={{ width: 12, height: 12, borderRadius: "50%", background: "#ff5f56" }} />
            <div style={{ width: 12, height: 12, borderRadius: "50%", background: "#ffbd2e" }} />
            <div style={{ width: 12, height: 12, borderRadius: "50%", background: "#27c93f" }} />
            <span style={{ fontSize: 13, color: "rgba(255,255,255,0.4)", marginLeft: 12, fontFamily: "sans-serif" }}>mrx-scanner@aphrody:~</span>
          </div>
          <pre style={{ margin: 0, lineHeight: "1.6", color: "#a8ffb2" }}>
            {`$ mrx scan --root .\n`}
            {frame > 10 && `[info] Scanning directories apps/web/src/ ...\n`}
            {frame > 20 && `total_files: 1513\n`}
            {frame > 35 && `scan_duration_ms: 47ms\n`}
            {frame > 50 && `languages:\n  CSS: 695 files\n  TypeScript: 673 files\n`}
            {frame > 65 && `[Frame Animation Step: ${Math.floor(frame)}]\n`}
            {frame > 80 && `mrx check ok. No violations found.`}
          </pre>
        </div>
      )}

      {/* Presentation view: waveforms and dynamic central visual */}
      {template === "presentation" && (
        <div
          style={{
            height: "45%",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 24,
          }}
        >
          <div
            style={{
              width: 120,
              height: 120,
              borderRadius: "50%",
              background: "rgba(255,255,255,0.1)",
              border: "2px solid rgba(255,255,255,0.2)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              boxShadow: "0 10px 30px rgba(0,0,0,0.2)",
              transform: `scale(${1 + Math.abs(Math.sin(frame * 0.1)) * 0.1})`,
            }}
          >
            <span style={{ fontSize: 48 }}>🎙️</span>
          </div>
        </div>
      )}

      {/* Audio Waveform Spectrum */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          marginTop: template === "presentation" ? 0 : 20,
          marginBottom: template === "presentation" ? 30 : 0,
          opacity: interpolate(frame, [0, 10], [0, 0.8], { extrapolateRight: "clamp" }),
        }}
      >
        {[...Array(waveBars)].map((_, i) => {
          // Animate height procedurally using frame offsets
          const phase = i * 0.4;
          const amp = Math.abs(Math.sin(frame * 0.15 + phase)) * (template === "presentation" ? 60 : 30) + 8;
          return (
            <div
              key={i}
              style={{
                width: template === "presentation" ? 8 : 5,
                height: amp,
                background: template === "presentation" ? "rgba(255,255,255,0.85)" : "rgba(255, 255, 255, 0.9)",
                borderRadius: 99,
                transition: "height 0.08s ease",
              }}
            />
          );
        })}
      </div>

      {/* Subtitles / Captions */}
      <div
        style={{
          position: "absolute",
          bottom: 50,
          left: "10%",
          right: "10%",
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "center",
          gap: "4px 8px",
          background: "rgba(0, 0, 0, 0.48)",
          backdropFilter: "blur(16px)",
          padding: "16px 24px",
          borderRadius: 24,
          border: "1px solid rgba(255,255,255,0.12)",
          boxShadow: "0 10px 30px rgba(0,0,0,0.25)",
        }}
      >
        {words.map((word, idx) => {
          const wordStartFrame = idx * framesPerWord;
          const wordEndFrame = (idx + 1) * framesPerWord;
          const isHighlighted = frame >= wordStartFrame && frame < wordEndFrame;
          return (
            <span
              key={idx}
              style={{
                fontSize: 22,
                fontWeight: 600,
                color: isHighlighted ? "#38ef7d" : "rgba(255,255,255,0.72)",
                transform: isHighlighted ? "scale(1.1)" : "scale(1)",
                transition: "all 0.12s cubic-bezier(0.16, 1, 0.3, 1)",
              }}
            >
              {word}
            </span>
          );
        })}
      </div>
    </AbsoluteFill>
  );
}
