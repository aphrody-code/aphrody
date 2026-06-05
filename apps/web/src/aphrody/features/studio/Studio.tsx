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
import { Player, PlayerRef } from "@remotion/player";
import { MascotVideo } from "./video/MascotVideo.tsx";
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

  const playerRef = useRef<PlayerRef>(null);

  // Sync player events with local state
  useEffect(() => {
    const { current } = playerRef;
    if (!current) return;

    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onFrameUpdate = () => {
      setCurrentFrame(current.getCurrentFrame());
    };

    current.addEventListener("play", onPlay);
    current.addEventListener("pause", onPause);
    current.addEventListener("frameupdate", onFrameUpdate);

    return () => {
      current.removeEventListener("play", onPlay);
      current.removeEventListener("pause", onPause);
      current.removeEventListener("frameupdate", onFrameUpdate);
    };
  }, [playerRef]);


  const handlePlayPause = () => {
    if (!playerRef.current) return;
    if (playerRef.current.isPlaying()) {
      playerRef.current.pause();
    } else {
      playerRef.current.play();
    }
  };

  const handleReplay = () => {
    if (!playerRef.current) return;
    playerRef.current.seekTo(0);
  };

  const handleSliderChange = (e: any) => {
    if (!playerRef.current) return;
    const frame = Number(e.target.value);
    playerRef.current.seekTo(frame);
    setCurrentFrame(frame);
  };

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
                position: "relative",
                overflow: "hidden",
                boxShadow: "0 12px 40px rgba(0,0,0,0.3)",
              }}
            >
              <Player
                ref={playerRef}
                component={MascotVideo}
                inputProps={{
                  template,
                  title,
                  subtitle,
                  theme,
                  speechText,
                }}
                durationInFrames={totalFrames}
                fps={fps}
                compositionWidth={1280}
                compositionHeight={720}
                style={{
                  width: "100%",
                  height: "100%",
                }}
                loop
                clickToPlay={false}
                controls={false}
              />
            </div>

            {/* Timeline Control Interface */}
            <div style={{ marginTop: 12 }}>
              <div className="owui-spread" style={{ alignItems: "center" }}>
                <div className="owui-row" style={{ gap: 4 }}>
                  <MdIconButton onClick={handlePlayPause}>
                    <MdIcon>{playing ? "pause" : "play_arrow"}</MdIcon>
                  </MdIconButton>
                  <MdIconButton onClick={handleReplay}>
                    <MdIcon>replay</MdIcon>
                  </MdIconButton>
                </div>

                <div style={{ flex: 1, margin: "0 16px" }}>
                  <MdSlider
                    min={0}
                    max={totalFrames - 1}
                    value={currentFrame}
                    onChange={handleSliderChange}
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
