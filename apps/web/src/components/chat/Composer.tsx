// Message composer: auto-sizing outlined textarea, attach + feature toggles
// (web search / image gen), and a send button that flips to Stop while streaming.
// Enter sends, Shift+Enter inserts a newline (open-webui's MessageInput behaviour).

import { useRef, useState } from "react";
import {
  MdFilterChip,
  MdIcon,
  MdIconButton,
  MdInputChip,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { useConfig } from "../../api/queries.ts";

export function Composer({
  value,
  onValue,
  onSend,
  onStop,
  streaming,
}: {
  value: string;
  onValue: (v: string) => void;
  onSend: () => void;
  onStop: () => void;
  streaming: boolean;
}) {
  const { data: config } = useConfig();
  const [webSearch, setWebSearch] = useState(false);
  const [imageGen, setImageGen] = useState(false);
  const [files, setFiles] = useState<string[]>([]);
  const fileRef = useRef<HTMLInputElement>(null);

  const canSend = value.trim().length > 0 && !streaming;

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (canSend) onSend();
    }
  };

  return (
    <div className="owui-composer">
      <div className="owui-composer__inner">
        {files.length > 0 && (
          <div className="owui-row" style={{ flexWrap: "wrap" }}>
            {files.map((f, i) => (
              <MdInputChip
                key={f + i}
                label={f}
                onRemove={() => setFiles((prev) => prev.filter((_, j) => j !== i))}
              />
            ))}
          </div>
        )}

        <div className="owui-composer__row">
          <MdIconButton aria-label="Joindre un fichier" onClick={() => fileRef.current?.click()}>
            <MdIcon>attach_file</MdIcon>
          </MdIconButton>
          <input
            ref={fileRef}
            type="file"
            hidden
            multiple
            onChange={(e) =>
              setFiles((prev) => [...prev, ...Array.from(e.target.files ?? []).map((f) => f.name)])
            }
          />

          <MdOutlinedTextField
            className="owui-composer__field"
            type="textarea"
            rows={1}
            placeholder="Envoyer un message…"
            value={value}
            onInput={(e) => onValue((e.target as HTMLInputElement).value)}
            onKeyDown={handleKey}
          />

          {streaming ? (
            <MdIconButton aria-label="Arrêter" onClick={onStop}>
              <MdIcon>stop_circle</MdIcon>
            </MdIconButton>
          ) : (
            <MdIconButton aria-label="Envoyer" disabled={!canSend} onClick={onSend}>
              <MdIcon>send</MdIcon>
            </MdIconButton>
          )}
        </div>

        <div className="owui-row" style={{ flexWrap: "wrap" }}>
          {config?.features.enable_web_search && (
            <MdFilterChip
              label="Recherche Web"
              selected={webSearch}
              onClick={() => setWebSearch((v) => !v)}
            />
          )}
          {config?.features.enable_image_generation && (
            <MdFilterChip
              label="Image"
              selected={imageGen}
              onClick={() => setImageGen((v) => !v)}
            />
          )}
        </div>
      </div>
    </div>
  );
}
