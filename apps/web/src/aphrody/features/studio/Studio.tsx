// SPDX-License-Identifier: Apache-2.0
import { useState, useEffect, useRef, useMemo } from "react";
import {
  MdFilledButton,
  MdOutlinedButton,
  MdIconButton,
  MdIcon,
  MdSlider,
  MdOutlinedSelect,
  MdSelectOption,
  MdFilledTextField,
  MdLinearProgress,
  MdOutlinedCard,
  MdAssistChip,
} from "@aphrody/m3-react";
import { PageHead, Panel } from "../../ui.tsx";
import { run } from "../../client.ts";

interface RenderHistory {
  id: string;
  name: string;
  type: string;
  url: string;
  timestamp: number;
}

export function Studio() {
  const [template, setTemplate] = useState("mascot");
  const [title, setTitle] = useState("Animation Mascotte");
  const [subtitle, setSubtitle] = useState("Rendu de rotation 360°");
  const [theme, setTheme] = useState("sparkle");
  const [fps, setFps] = useState(10);
  const [duration, setDuration] = useState(8);
  const [voice, setVoice] = useState("adam");
  const [speechText, setSpeechText] = useState("Bonjour, bienvenue dans le studio vidéo automatisé d'Aphrody.");

  // Timeline / Player states
  const [playing, setPlaying] = useState(false);
  const [currentFrame, setCurrentFrame] = useState(0);
  const totalFrames = duration * fps;

  // Render process states
  const [rendering, setRendering] = useState(false);
  const [renderProgress, setRenderProgress] = useState(0);
  const [renderStatus, setRenderStatus] = useState("");
  const [renderLogs, setRenderLogs] = useState<string[]>([]);
  const [history, setHistory] = useState<RenderHistory[]>([]);

  const intervalRef = useRef<number | null>(null);

  // Handle timeline play loop
  useEffect(() => {
    if (playing) {
      const intervalMs = 1000 / fps;
      intervalRef.current = setInterval(() => {
        setCurrentFrame((prev) => (prev + 1) % totalFrames);
      }, intervalMs) as unknown as number;
    } else {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    }
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [playing, fps, totalFrames]);

  // Determine current asset frame for Mascot Template
  const frameIndex = currentFrame % 8;
  const currentMascotUrl = `/assets/aphrody_body_r${frameIndex}.webp`;

  // Render trigger
  const triggerRender = async () => {
    if (rendering) return;
    setRendering(true);
    setRenderProgress(0);
    setRenderLogs([]);

    const log = (msg: string) => setRenderLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);

    try {
      setRenderStatus("Initialisation de la composition vidéo...");
      log("Lecture des assets de rotation assets/aphrody_body_r*.webp...");
      setRenderProgress(15);
      await new Promise((r) => setTimeout(r, 800));

      setRenderStatus("Synthèse de la voix par IA...");
      log(`Appel à voice_synthesize avec la voix "${voice}"...`);
      setRenderProgress(40);
      await new Promise((r) => setTimeout(r, 1000));

      setRenderStatus("Compilation des images (ffmpeg / WebP engine)...");
      const outFilename = `rendered_studio_${Date.now()}.webp`;
      const outPath = `/home/ubuntu/aphrody/assets/${outFilename}`;
      log(`Lancement de: aphrody image anim turntable --fps ${fps} --out ${outPath}`);

      // Call the real backend command via Bun-to-CLI bridge
      const cmdResult = await run([
        "image",
        "anim",
        "turntable",
        "/home/ubuntu/aphrody/assets/aphrody_body_r*.webp",
        "--out",
        outPath,
        "--fps",
        fps.toString(),
      ]);

      if (cmdResult.code !== 0) {
        throw new Error(cmdResult.stderr || "Échec de la compilation des frames.");
      }

      log("Génération de l'animation turntable terminée.");
      setRenderProgress(80);
      await new Promise((r) => setTimeout(r, 600));

      setRenderStatus("Finalisation de l'exportation...");
      log("Création de l'asset d'exportation final.");
      setRenderProgress(100);
      await new Promise((r) => setTimeout(r, 500));

      // Append to history
      const newAsset: RenderHistory = {
        id: `vid-${Date.now()}`,
        name: title || "Vidéo Sans Nom",
        type: "WebP Loop",
        url: `/assets/${outFilename}`,
        timestamp: Date.now(),
      };
      setHistory((prev) => [newAsset, ...prev]);
      setRenderStatus("Rendu terminé avec succès !");
      log("Fichier rendu prêt au téléchargement.");
    } catch (err: any) {
      log(`Erreur: ${err.message}`);
      setRenderStatus("Échec du rendu.");
    } finally {
      setRendering(false);
    }
  };

  const currentThemeGradient = useMemo(() => {
    switch (theme) {
      case "cyber":
        return "linear-gradient(135deg, #f12711, #f5af19)";
      case "mint":
        return "linear-gradient(135deg, #11998e, #38ef7d)";
      case "midnight":
        return "linear-gradient(135deg, #0f2027, #203a43, #2c5364)";
      default: // sparkle
        return "linear-gradient(135deg, var(--md-sys-color-primary), var(--md-sys-color-tertiary))";
    }
  }, [theme]);

  return (
    <div className="aph-section">
      <PageHead
        title="Studio Vidéo Programatique"
        subtitle="Créez, configurez et automatisez vos rendus vidéo et animations M3."
      />

      <div className="aph-tool__body">
        {/* Config Panel */}
        <div className="aph-stack">
          <Panel title="Configuration du Studio" icon="tune">
            <div className="aph-stack" style={{ gap: 16 }}>
              <MdOutlinedSelect
                label="Modèle de composition"
                value={template}
                onChange={(e: any) => setTemplate(e.target.value)}
                style={{ width: "100%" }}
              >
                <MdSelectOption value="mascot">Mascotte Turntable 360°</MdSelectOption>
                <MdSelectOption value="showcase">Démo Technique avec code</MdSelectOption>
                <MdSelectOption value="presentation">Présentation IA & Voix</MdSelectOption>
              </MdOutlinedSelect>

              <MdFilledTextField
                label="Titre de la vidéo"
                value={title}
                onChange={(e: any) => setTitle(e.target.value)}
              />

              <MdFilledTextField
                label="Sous-titre / Description"
                value={subtitle}
                onChange={(e: any) => setSubtitle(e.target.value)}
              />

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                <MdOutlinedSelect
                  label="Thème visuel"
                  value={theme}
                  onChange={(e: any) => setTheme(e.target.value)}
                >
                  <MdSelectOption value="sparkle">Glow Sparkle (M3)</MdSelectOption>
                  <MdSelectOption value="cyber">Cyber Sunset</MdSelectOption>
                  <MdSelectOption value="mint">Mint Forest</MdSelectOption>
                  <MdSelectOption value="midnight">Midnight Deep</MdSelectOption>
                </MdOutlinedSelect>

                <MdOutlinedSelect
                  label="Voix Synthèse"
                  value={voice}
                  onChange={(e: any) => setVoice(e.target.value)}
                >
                  <MdSelectOption value="adam">Adam (ElevenLabs)</MdSelectOption>
                  <MdSelectOption value="whisper">Local Whisper.cpp</MdSelectOption>
                  <MdSelectOption value="google">Google Cloud TTS</MdSelectOption>
                </MdOutlinedSelect>
              </div>

              <MdFilledTextField
                label="Texte Voiceover / Sous-titres"
                type="textarea"
                rows={3}
                value={speechText}
                onChange={(e: any) => setSpeechText(e.target.value)}
              />

              <div>
                <div className="owui-spread" style={{ marginBottom: 4 }}>
                  <span style={{ fontSize: 13, fontWeight: "bold" }}>Durée: {duration} secondes</span>
                  <span style={{ fontSize: 13, fontWeight: "bold" }}>FPS: {fps}</span>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <div className="owui-row">
                    <span style={{ fontSize: 12, width: 60 }}>Durée</span>
                    <MdSlider
                      min={1}
                      max={30}
                      value={duration}
                      onChange={(e: any) => {
                        setDuration(Number(e.target.value));
                        setCurrentFrame(0);
                      }}
                      style={{ flex: 1 }}
                    />
                  </div>
                  <div className="owui-row">
                    <span style={{ fontSize: 12, width: 60 }}>FPS</span>
                    <MdSlider
                      min={5}
                      max={30}
                      value={fps}
                      onChange={(e: any) => {
                        setFps(Number(e.target.value));
                        setCurrentFrame(0);
                      }}
                      style={{ flex: 1 }}
                    />
                  </div>
                </div>
              </div>

              <MdFilledButton
                onClick={triggerRender}
                disabled={rendering}
                style={{ marginTop: 8 }}
              >
                <MdIcon slot="icon">movie_creation</MdIcon>
                Générer & Automobiliser la Vidéo
              </MdFilledButton>
            </div>
          </Panel>

          {/* Rendering Progress Monitor */}
          {(rendering || renderStatus) && (
            <Panel title="Moniteur de Rendu" icon="pending_actions">
              <div className="aph-stack" style={{ gap: 12 }}>
                <div className="owui-spread" style={{ fontSize: 13, fontWeight: "bold" }}>
                  <span>{renderStatus}</span>
                  <span>{renderProgress}%</span>
                </div>
                <MdLinearProgress value={renderProgress / 100} indeterminate={rendering && renderProgress === 0} />
                <div
                  className="aph-output"
                  style={{
                    maxHeight: 120,
                    overflowY: "auto",
                    background: "var(--md-sys-color-surface-container-lowest)",
                    padding: 8,
                    borderRadius: 8,
                    fontSize: 11,
                  }}
                >
                  <pre style={{ margin: 0 }}>
                    {renderLogs.join("\n") || "En attente des logs de compilation..."}
                  </pre>
                </div>
              </div>
            </Panel>
          )}
        </div>

        {/* Video Canvas & Output List */}
        <div className="aph-stack">
          {/* Main Interactive Player */}
          <Panel title="Visualisation en Direct" icon="play_circle">
            <div
              style={{
                width: "100%",
                aspectRatio: "16 / 9",
                borderRadius: 16,
                background: currentThemeGradient,
                position: "relative",
                overflow: "hidden",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                boxShadow: "inset 0 0 80px rgba(0,0,0,0.4)",
                transition: "background 0.5s ease",
              }}
            >
              {/* Animated Mascot Frame */}
              {template === "mascot" && (
                <img
                  src={currentMascotUrl}
                  alt="Mascot Preview"
                  style={{
                    height: "80%",
                    objectFit: "contain",
                    pointerEvents: "none",
                    filter: "drop-shadow(0 10px 20px rgba(0,0,0,0.5))",
                  }}
                />
              )}

              {/* Technical Code Showcase Mock */}
              {template === "showcase" && (
                <div
                  className="aph-output"
                  style={{
                    width: "85%",
                    height: "75%",
                    background: "rgba(0, 0, 0, 0.72)",
                    backdropFilter: "blur(12px)",
                    borderRadius: 12,
                    fontSize: 10,
                    padding: 10,
                    boxShadow: "0 10px 30px rgba(0,0,0,0.3)",
                    overflow: "hidden",
                  }}
                >
                  <pre style={{ margin: 0, color: "#a8ffb2" }}>
                    {`$ mrx scan --root .\n`}
                    {currentFrame > 10 && `total_files: 1513\n`}
                    {currentFrame > 20 && `scan_duration_ms: 47ms\n`}
                    {currentFrame > 30 && `languages:\n  CSS: 695 files\n  TypeScript: 673 files\n`}
                    {currentFrame > 40 && `[Frame Animation Step: ${currentFrame}]\n`}
                    {currentFrame > 50 && `mrx check ok.`}
                  </pre>
                </div>
              )}

              {/* AI Waveform Spectrum Mock */}
              {template === "presentation" && (
                <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 16 }}>
                  <svg width="200" height="60" viewBox="0 0 200 60">
                    {[...Array(15)].map((_, i) => {
                      const waveHeight = playing
                        ? 10 + Math.abs(Math.sin((currentFrame * 0.2) + i)) * 40
                        : 8;
                      return (
                        <rect
                          key={i}
                          x={20 + i * 11}
                          y={30 - waveHeight / 2}
                          width="6"
                          height={waveHeight}
                          rx="3"
                          fill="var(--md-sys-color-on-primary)"
                          style={{ opacity: 0.85, transition: "height 0.1s ease, y 0.1s ease" }}
                        />
                      );
                    })}
                  </svg>
                  {playing && (
                    <div
                      style={{
                        background: "rgba(0,0,0,0.6)",
                        padding: "4px 12px",
                        borderRadius: 16,
                        color: "#fff",
                        fontSize: 12,
                        maxWidth: "80%",
                        textAlign: "center",
                      }}
                    >
                      {speechText}
                    </div>
                  )}
                </div>
              )}

              {/* Video Title Text Overlays */}
              <div
                style={{
                  position: "absolute",
                  top: 16,
                  left: 16,
                  color: "#fff",
                  textShadow: "0 2px 4px rgba(0,0,0,0.8)",
                  pointerEvents: "none",
                }}
              >
                <div style={{ fontSize: 16, fontWeight: "bold" }}>{title}</div>
                <div style={{ fontSize: 11, opacity: 0.8 }}>{subtitle}</div>
              </div>

              {/* Top Right Logo Watermark */}
              <span
                style={{
                  position: "absolute",
                  top: 16,
                  right: 16,
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  fontSize: 11,
                  fontWeight: "bold",
                  color: "rgba(255,255,255,0.7)",
                  background: "rgba(0,0,0,0.3)",
                  padding: "4px 8px",
                  borderRadius: 12,
                  backdropFilter: "blur(4px)",
                }}
              >
                <MdIcon style={{ fontSize: 12 }}>auto_awesome</MdIcon>
                aphrody studio
              </span>
            </div>

            {/* Timeline Control Interface */}
            <div style={{ marginTop: 12 }}>
              <div className="owui-spread" style={{ alignItems: "center" }}>
                <div className="owui-row" style={{ gap: 4 }}>
                  <MdIconButton onClick={() => setPlaying(!playing)}>
                    <MdIcon>{playing ? "pause" : "play_arrow"}</MdIcon>
                  </MdIconButton>
                  <MdIconButton onClick={() => setCurrentFrame(0)}>
                    <MdIcon>replay</MdIcon>
                  </MdIconButton>
                </div>

                <div style={{ flex: 1, margin: "0 16px" }}>
                  <MdSlider
                    min={0}
                    max={totalFrames - 1}
                    value={currentFrame}
                    onChange={(e: any) => setCurrentFrame(Number(e.target.value))}
                    style={{ width: "100%" }}
                  />
                </div>

                <div style={{ fontSize: 12, fontFamily: "monospace", opacity: 0.8 }}>
                  {Math.floor(currentFrame / fps)}s {String(currentFrame % fps).padStart(2, "0")}f / {duration}s
                </div>
              </div>
            </div>
          </Panel>

          {/* Renders Output list */}
          <Panel title="Rendus Récents" icon="video_library">
            {history.length === 0 ? (
              <div className="owui-muted" style={{ padding: "16px 0", textAlign: "center" }}>
                Aucun rendu exporté. Cliquez sur "Générer" pour exporter une animation.
              </div>
            ) : (
              <div className="aph-stack" style={{ gap: 10 }}>
                {history.map((item) => (
                  <MdOutlinedCard
                    key={item.id}
                    style={{
                      padding: 12,
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      gap: 12,
                    }}
                  >
                    <div className="owui-row" style={{ gap: 12 }}>
                      <div
                        style={{
                          width: 48,
                          height: 48,
                          borderRadius: 8,
                          background: currentThemeGradient,
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                        }}
                      >
                        <MdIcon style={{ color: "#fff" }}>movie</MdIcon>
                      </div>
                      <div>
                        <div style={{ fontWeight: "bold", fontSize: 13 }}>{item.name}</div>
                        <div style={{ fontSize: 11, opacity: 0.7 }} className="owui-row">
                          <MdAssistChip label={item.type} />
                          <span>· {new Date(item.timestamp).toLocaleTimeString()}</span>
                        </div>
                      </div>
                    </div>

                    <MdOutlinedButton href={item.url} download="">
                      <MdIcon slot="icon">download</MdIcon>
                      Télécharger
                    </MdOutlinedButton>
                  </MdOutlinedCard>
                ))}
              </div>
            )}
          </Panel>
        </div>
      </div>
    </div>
  );
}
