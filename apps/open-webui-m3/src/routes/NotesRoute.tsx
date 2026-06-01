// "/notes" — minimal notes list + inline editor (M3 cards + outlined textarea).
// open-webui's notes area, trimmed to the essentials.

import { useState } from "react";
import {
  MdFilledTonalButton,
  MdIcon,
  MdIconButton,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";

interface Note {
  id: string;
  title: string;
  body: string;
}

const SEED: Note[] = [
  { id: "n-1", title: "Meeting notes", body: "Ship the M3 rebuild. Bun + TanStack only." },
  { id: "n-2", title: "Ideas", body: "Add shared-axis transitions between routes." },
];

export function NotesRoute() {
  const [notes, setNotes] = useState<Note[]>(SEED);
  const [activeId, setActiveId] = useState<string>(SEED[0].id);
  const active = notes.find((n) => n.id === activeId);

  const addNote = () => {
    const id = `n-${notes.length + 1}-${Date.now()}`;
    const note: Note = { id, title: "Untitled", body: "" };
    setNotes((prev) => [note, ...prev]);
    setActiveId(id);
  };

  const patch = (p: Partial<Note>) =>
    setNotes((prev) => prev.map((n) => (n.id === activeId ? { ...n, ...p } : n)));

  return (
    <div className="owui-page">
      <div
        className="owui-page__inner"
        style={{ display: "grid", gridTemplateColumns: "260px 1fr", gap: 16 }}
      >
        <div>
          <div className="owui-spread" style={{ marginBottom: 8 }}>
            <h2 style={{ margin: 0 }}>Notes</h2>
            <MdFilledTonalButton onClick={addNote}>
              <MdIcon slot="icon">add</MdIcon>New
            </MdFilledTonalButton>
          </div>
          <div className="owui-stack">
            {notes.map((n) => (
              <MdOutlinedCard
                key={n.id}
                onClick={() => setActiveId(n.id)}
                style={{
                  padding: 12,
                  cursor: "pointer",
                  outline: n.id === activeId ? "2px solid var(--md-sys-color-primary)" : "none",
                }}
              >
                <strong>{n.title || "Untitled"}</strong>
                <p
                  className="owui-muted"
                  style={{
                    margin: "4px 0 0",
                    fontSize: 13,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {n.body || "Empty note"}
                </p>
              </MdOutlinedCard>
            ))}
          </div>
        </div>

        {active && (
          <div className="owui-stack">
            <div className="owui-spread">
              <MdOutlinedTextField
                label="Title"
                value={active.title}
                onInput={(e) => patch({ title: (e.target as HTMLInputElement).value })}
                style={{ flex: 1 }}
              />
              <MdIconButton
                aria-label="Delete note"
                onClick={() => {
                  setNotes((prev) => prev.filter((n) => n.id !== active.id));
                  const rest = notes.filter((n) => n.id !== active.id);
                  if (rest[0]) setActiveId(rest[0].id);
                }}
              >
                <MdIcon>delete</MdIcon>
              </MdIconButton>
            </div>
            <MdOutlinedTextField
              type="textarea"
              rows={14}
              label="Note"
              value={active.body}
              onInput={(e) => patch({ body: (e.target as HTMLInputElement).value })}
              style={{ width: "100%" }}
            />
          </div>
        )}
      </div>
    </div>
  );
}
