// SPDX-License-Identifier: Apache-2.0
"use client";

import { useCallback, useRef, useState, type JSX } from "react";

import { AppBar } from "./components/AppBar";
import { ConversationPane } from "./components/ConversationPane";
import { HeroEmptyState } from "./components/HeroEmptyState";
import { LeftRail, type RailKey } from "./components/LeftRail";
import { SettingsDrawer } from "./components/SettingsDrawer";
import { ShareDialog } from "./components/ShareDialog";
import { GemsPanel } from "./components/GemsPanel";
import { RecentHistory } from "./components/RecentHistory";
import {
  PromptBar,
  type PromptBarHandle,
} from "./components/PromptBar";
import type { GeminiModelId } from "./components/ModelPicker";
import { useChat } from "../hooks/useChat";
import { useVoiceOutput } from "../hooks/useVoiceOutput";

export default function GeminiAppPage(): JSX.Element {
  const [model, setModel] = useState<GeminiModelId>("gemini-2.5-flash");
  const [rail, setRail] = useState<RailKey>("new");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const { turns, busy, error, send } = useChat();
  const { speak, cancel: cancelVoice, autoTTS, setAutoTTS } = useVoiceOutput();
  const promptRef = useRef<PromptBarHandle | null>(null);

  const onSuggest = useCallback((text: string) => {
    promptRef.current?.setText(text);
  }, []);

  const onSubmit = useCallback(
    async (text: string) => {
      cancelVoice();
      await send(text);
      // Fire-and-forget TTS playback of the latest assistant reply, if any.
      // We pluck it from the resolved turns array via a microtask hop.
      queueMicrotask(() => {
        if (!autoTTS) return;
        const last = turns[turns.length - 1];
        if (last && last.role === "assistant" && !last.streaming) {
          void speak(last.text);
        }
      });
    },
    [cancelVoice, send, speak, turns, autoTTS],
  );

  const hasMessages = turns.length > 0;

  return (
    <div className="app">
      <AppBar model={model} onModelChange={setModel} onOpenSettings={() => setSettingsOpen(true)} onOpenShare={() => setShareOpen(true)} />
      <LeftRail active={rail} onSelect={setRail} />
      <GemsPanel open={rail === "gems"} onClose={() => setRail("new")} />
      <RecentHistory open={rail === "recent"} onClose={() => setRail("new")} />
      <main className="main" role="main">
        <section className={`chat${hasMessages ? " has-messages" : ""}`}>
          {hasMessages ? null : <HeroEmptyState onSuggest={onSuggest} />}
          <ConversationPane turns={turns} />
          {error ? (
            <div
              role="alert"
              style={{
                margin: "8px auto",
                padding: "8px 14px",
                borderRadius: 12,
                background: "color-mix(in srgb, #EC4899 14%, transparent)",
                color: "#7A1B4A",
                fontSize: 13,
                maxWidth: 720,
              }}
            >
              {error}
            </div>
          ) : null}
        </section>
        <PromptBar ref={promptRef} busy={busy} onSubmit={onSubmit} />
      </main>
      <SettingsDrawer 
        open={settingsOpen || rail === "settings"} 
        onClose={() => { setSettingsOpen(false); setRail("new"); }} 
        autoTTS={autoTTS} 
        onAutoTTSChange={setAutoTTS} 
      />
      <ShareDialog
        open={shareOpen}
        onClose={() => setShareOpen(false)}
      />
    </div>
  );
}
