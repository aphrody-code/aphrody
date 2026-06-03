// Assistant chat surface (React port of Angular AssistantComponent): Gemini-style hero + composer + conversation, sending runs `aphrody chat --prompt`.

import { useCallback, useEffect, useRef, useState } from "react";
import { MdAssistChip, MdChipSet, MdIcon, MdIconButton, MdMenuItem } from "@aphrody/m3-react";
import { run } from "../../client.ts";
import { Menu } from "../../../components/ui/Menu.tsx";
import { VoiceOverlay } from "./VoiceOverlay.tsx";

function voiceSupported(): boolean {
  if (typeof window === "undefined") return false;
  const w = window as unknown as { SpeechRecognition?: unknown; webkitSpeechRecognition?: unknown };
  return Boolean(w.SpeechRecognition ?? w.webkitSpeechRecognition);
}

// ── Attachment model (ported from the Angular `attachment.ts` helper) ────────

type AttachmentKind = "image" | "video" | "audio" | "github" | "url" | "drive" | "file";

interface Attachment {
  id: string;
  kind: AttachmentKind;
  name: string;
  detail?: string;
  text?: string;
  loading?: boolean;
  note?: string;
}

function kindIcon(kind: AttachmentKind): string {
  switch (kind) {
    case "image":
      return "image";
    case "video":
      return "movie";
    case "audio":
      return "graphic_eq";
    case "github":
      return "code";
    case "url":
      return "link";
    case "drive":
      return "cloud";
    default:
      return "description";
  }
}

function kindLabel(kind: AttachmentKind): string {
  switch (kind) {
    case "image":
      return "Image";
    case "video":
      return "Vidéo";
    case "audio":
      return "Audio";
    case "github":
      return "GitHub";
    case "url":
      return "Lien";
    case "drive":
      return "Drive";
    default:
      return "Fichier";
  }
}

const TEXT_EXT = new Set([
  "txt",
  "md",
  "markdown",
  "rs",
  "ts",
  "tsx",
  "js",
  "jsx",
  "mjs",
  "cjs",
  "json",
  "jsonc",
  "toml",
  "yaml",
  "yml",
  "html",
  "htm",
  "css",
  "scss",
  "sass",
  "less",
  "py",
  "go",
  "java",
  "kt",
  "kts",
  "c",
  "h",
  "cpp",
  "hpp",
  "cc",
  "cs",
  "rb",
  "php",
  "sh",
  "bash",
  "zsh",
  "ps1",
  "sql",
  "xml",
  "svg",
  "csv",
  "tsv",
  "ini",
  "cfg",
  "conf",
  "env",
  "lock",
  "gradle",
  "dockerfile",
  "vue",
  "svelte",
  "astro",
  "graphql",
  "proto",
  "lua",
  "r",
  "swift",
  "dart",
  "ex",
  "exs",
  "log",
]);

function isTextFile(name: string, mime: string): boolean {
  if (mime.startsWith("text/")) return true;
  if (mime === "application/json" || mime === "application/xml") return true;
  const dot = name.lastIndexOf(".");
  const ext = dot >= 0 ? name.slice(dot + 1).toLowerCase() : name.toLowerCase();
  return TEXT_EXT.has(ext);
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  const ko = bytes / 1024;
  if (ko < 1024) return `${ko.toFixed(1).replace(".", ",")} ko`;
  return `${(ko / 1024).toFixed(1).replace(".", ",")} Mo`;
}

const MAX_INLINE_CHARS = 24_000;

function githubRawCandidates(input: string): string[] {
  let url: URL;
  try {
    url = new URL(input.trim());
  } catch {
    return [];
  }
  if (url.hostname !== "github.com") return [];
  const parts = url.pathname.split("/").filter(Boolean);
  if (parts.length >= 5 && parts[2] === "blob") {
    const [owner, repo, , ref, ...rest] = parts;
    return [`https://raw.githubusercontent.com/${owner}/${repo}/${ref}/${rest.join("/")}`];
  }
  if (parts.length >= 2) {
    const [owner, repo] = parts;
    const branches = ["main", "master"];
    const names = ["README.md", "readme.md", "README.MD"];
    const out: string[] = [];
    for (const b of branches) {
      for (const n of names) {
        out.push(`https://raw.githubusercontent.com/${owner}/${repo}/${b}/${n}`);
      }
    }
    return out;
  }
  return [];
}

function htmlToText(html: string): string {
  const noScript = html
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<!--[\s\S]*?-->/g, " ");
  const text = noScript.replace(/<[^>]+>/g, " ");
  const decoded = text
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'");
  return decoded
    .replace(/[ \t]+/g, " ")
    .replace(/\n\s*\n\s*\n+/g, "\n\n")
    .trim();
}

// ── Conversation + suggestions ───────────────────────────────────────────────

interface Turn {
  role: "user" | "assistant";
  text: string;
  pending?: boolean;
}

interface Suggestion {
  icon: string;
  label: string;
  prompt: string;
}

// Gemini 3.5 Flash is the chat model, served by the keyless Gemini web transport
// (the default backend). The id is passed to `aphrody chat --model`.
const MODELS = [{ id: "gemini-3.5-flash", label: "Gemini 3.5 Flash" }] as const;

const ACCEPT: Record<"image" | "video" | "audio", string> = {
  image: "image/*",
  video: "video/*",
  audio: "audio/*",
};

const SUGGESTIONS: Suggestion[] = [
  {
    icon: "edit",
    label: "Rédiger un message",
    prompt: "Aide-moi à rédiger un e-mail professionnel et clair.",
  },
  {
    icon: "school",
    label: "Expliquer un sujet",
    prompt: "Explique-moi un concept compliqué avec des mots simples.",
  },
  {
    icon: "lightbulb",
    label: "Trouver des idées",
    prompt: "Propose-moi des idées originales pour un projet.",
  },
  {
    icon: "summarize",
    label: "Résumer un texte",
    prompt: "Résume ce texte en quelques points clés : ",
  },
];

let idCounter = 0;
function nextId(): string {
  idCounter += 1;
  return `att-${idCounter}`;
}

export function Assistant() {
  const [turns, setTurns] = useState<Turn[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [model] = useState<(typeof MODELS)[number]>(MODELS[0]);
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [linkPanel, setLinkPanel] = useState<"github" | "url" | "drive" | null>(null);
  const [linkValue, setLinkValue] = useState("");
  const [voiceMode, setVoiceMode] = useState(false);
  const speechSupported = voiceSupported();

  const scrollPane = useRef<HTMLDivElement>(null);
  const promptInput = useRef<HTMLTextAreaElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const pickerKind = useRef<AttachmentKind>("file");
  const busyRef = useRef(false);

  const hasConversation = turns.length > 0;

  useEffect(() => {
    const el = scrollPane.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [turns]);

  const autoGrow = useCallback(() => {
    const el = promptInput.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  }, []);

  const onInput = (value: string) => {
    setDraft(value);
    autoGrow();
  };

  // ── Attachment patching ────────────────────────────────────────────────────

  const patchAttachment = useCallback((id: string, patch: Partial<Attachment>) => {
    setAttachments((list) => list.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  }, []);

  const removeAttachment = (id: string) => {
    setAttachments((a) => a.filter((x) => x.id !== id));
  };

  const fetchInto = useCallback(
    async (id: string, urls: string[], stripHtml = false): Promise<void> => {
      for (const u of urls) {
        try {
          const res = await fetch(u, { redirect: "follow" });
          if (!res.ok) continue;
          let body = await res.text();
          const ct = res.headers.get("content-type") ?? "";
          if (stripHtml || ct.includes("text/html")) {
            body = htmlToText(body);
          }
          body = body.trim();
          if (!body) continue;
          const truncated = body.length > MAX_INLINE_CHARS;
          patchAttachment(id, {
            loading: false,
            text: truncated ? body.slice(0, MAX_INLINE_CHARS) : body,
            note: truncated ? "contenu tronqué (trop long)" : undefined,
          });
          return;
        } catch {
          // try the next candidate
        }
      }
      patchAttachment(id, {
        loading: false,
        note: "contenu non récupérable (réseau bloqué ou ressource privée) — lien tout de même référencé",
      });
    },
    [patchAttachment],
  );

  // ── Native file picker ──────────────────────────────────────────────────────

  const pickMedia = (kind: "image" | "video" | "audio" | "file") => {
    pickerKind.current = kind;
    const input = fileInput.current;
    if (!input) return;
    input.accept = kind === "file" ? "" : ACCEPT[kind];
    input.value = "";
    input.click();
  };

  const onFilesChosen = async (ev: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(ev.target.files ?? []);
    const kind = pickerKind.current;
    for (const file of files) {
      const textual = kind === "file" && isTextFile(file.name, file.type);
      const att: Attachment = {
        id: nextId(),
        kind,
        name: file.name,
        detail: `${formatBytes(file.size)}${file.type ? ` · ${file.type}` : ""}`,
        loading: textual,
      };
      setAttachments((a) => [...a, att]);
      if (textual) {
        try {
          const raw = await file.text();
          const text = raw.length > MAX_INLINE_CHARS ? raw.slice(0, MAX_INLINE_CHARS) : raw;
          patchAttachment(att.id, {
            loading: false,
            text,
            note: raw.length > MAX_INLINE_CHARS ? "contenu tronqué (trop long)" : undefined,
          });
        } catch (err) {
          patchAttachment(att.id, { loading: false, note: `lecture impossible : ${String(err)}` });
        }
      } else {
        patchAttachment(att.id, {
          note: "média binaire référencé (chat texte uniquement — pas d'analyse multimodale)",
        });
      }
    }
  };

  const addGithub = useCallback(
    async (url: string) => {
      const trimmed = url.trim();
      if (!trimmed) return;
      const candidates = githubRawCandidates(trimmed);
      const att: Attachment = {
        id: nextId(),
        kind: "github",
        name: trimmed,
        detail: "GitHub",
        loading: candidates.length > 0,
      };
      setAttachments((a) => [...a, att]);
      if (candidates.length === 0) {
        patchAttachment(att.id, {
          loading: false,
          note: "URL GitHub non reconnue (attendu : lien blob ou dépôt)",
        });
        return;
      }
      await fetchInto(att.id, candidates);
    },
    [fetchInto, patchAttachment],
  );

  const addUrl = useCallback(
    async (url: string) => {
      const trimmed = url.trim();
      if (!trimmed) return;
      let normalized = trimmed;
      if (!/^https?:\/\//i.test(normalized)) normalized = `https://${normalized}`;
      let host = normalized;
      try {
        host = new URL(normalized).hostname;
      } catch {
        // keep the raw string as detail
      }
      const att: Attachment = {
        id: nextId(),
        kind: "url",
        name: trimmed,
        detail: host,
        loading: true,
      };
      setAttachments((a) => [...a, att]);
      await fetchInto(att.id, [normalized], true);
    },
    [fetchInto],
  );

  const addDrive = (link: string) => {
    const trimmed = link.trim();
    if (!trimmed) return;
    setAttachments((a) => [
      ...a,
      {
        id: nextId(),
        kind: "drive",
        name: trimmed,
        detail: "Google Drive (lien)",
        note: "lien référencé tel quel (aucune authentification Drive)",
      },
    ]);
  };

  // ── Link panel ──────────────────────────────────────────────────────────────

  const openLinkPanel = (kind: "github" | "url" | "drive") => {
    setLinkValue("");
    setLinkPanel(kind);
  };
  const closeLinkPanel = () => setLinkPanel(null);

  const linkPlaceholder = (): string => {
    switch (linkPanel) {
      case "github":
        return "https://github.com/owner/repo ou .../blob/main/fichier.rs";
      case "drive":
        return "https://drive.google.com/file/d/…";
      default:
        return "https://exemple.com/article";
    }
  };

  const submitLink = () => {
    const kind = linkPanel;
    const v = linkValue.trim();
    if (kind && v) {
      if (kind === "github") void addGithub(v);
      else if (kind === "url") void addUrl(v);
      else addDrive(v);
    }
    closeLinkPanel();
  };

  // ── Prompt context + send ───────────────────────────────────────────────────

  const buildContext = useCallback((items: Attachment[]): string => {
    if (items.length === 0) return "";
    const blocks: string[] = [];
    for (const a of items) {
      const header = `[${kindLabel(a.kind)}] ${a.name}`;
      if (a.text) {
        blocks.push(`${header}\n\`\`\`\n${a.text}\n\`\`\``);
      } else {
        const reason = a.note ?? "référencé sans contenu textuel";
        blocks.push(`${header} (${reason})`);
      }
    }
    return `Contexte fourni par l'utilisateur :\n\n${blocks.join("\n\n")}`;
  }, []);

  const revealReply = useCallback(async (full: string, idx: number) => {
    const step = Math.max(1, Math.round(full.length / 240));
    for (let i = 0; i <= full.length; i += step) {
      const slice = full.slice(0, i);
      const pending = i < full.length;
      setTurns((t) => {
        const copy = [...t];
        copy[idx] = { role: "assistant", text: slice, pending };
        return copy;
      });
      await new Promise((r) => setTimeout(r, 12));
    }
    setTurns((t) => {
      const copy = [...t];
      copy[idx] = { role: "assistant", text: full, pending: false };
      return copy;
    });
  }, []);

  const send = useCallback(async () => {
    const prompt = draft.trim();
    const current = attachments;
    const hasAtt = current.length > 0;
    if ((!prompt && !hasAtt) || busyRef.current) return;
    busyRef.current = true;
    setBusy(true);

    const context = buildContext(current);
    const userVisible = prompt || "(voir le contexte joint)";
    const fullPrompt = context ? `${context}\n\n---\n\n${prompt}` : prompt;

    setDraft("");
    setAttachments([]);
    if (promptInput.current) promptInput.current.style.height = "auto";

    let assistantIdx = -1;
    setTurns((t) => {
      assistantIdx = t.length + 1;
      return [
        ...t,
        { role: "user", text: userVisible },
        { role: "assistant", text: "", pending: true },
      ];
    });

    let reply: string;
    try {
      // `--web` is the default chat transport (the only one serving real Gemini
      // 3.5 Flash); the CLI falls back to the agy token when the cookie jar is absent.
      const res = await run(["chat", "--prompt", fullPrompt, "--model", model.id, "--web"]);
      reply = (res.stdout || res.stderr || "").trim() || "(réponse vide)";
      if (res.code !== 0 && !res.stdout) {
        reply = `La commande a échoué (code ${res.code}).\n\n${res.stderr}`.trim();
      }
    } catch (err) {
      reply = `Erreur lors de l'appel au backend aphrody : ${String(err)}`;
    }

    await revealReply(reply, assistantIdx);
    busyRef.current = false;
    setBusy(false);
  }, [draft, attachments, model.id, buildContext, revealReply]);

  const onKeydown = (ev: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (ev.key === "Enter" && !ev.shiftKey) {
      ev.preventDefault();
      void send();
    }
  };

  const useSuggestion = (s: Suggestion) => {
    setDraft(s.prompt);
  };

  const newChat = () => {
    setTurns([]);
    setDraft("");
    setAttachments([]);
    busyRef.current = false;
    setBusy(false);
  };

  const canSend = draft.trim().length > 0 || attachments.length > 0;

  return (
    <div className="aph-assistant">
      {hasConversation ? (
        <div className="aph-assistant__conversation" ref={scrollPane}>
          <div className="aph-assistant__conversation-inner">
            {turns.map((turn, i) =>
              turn.role === "user" ? (
                <div key={i} className="aph-turn aph-turn--user">
                  <div className="aph-bubble aph-bubble--user">{turn.text}</div>
                </div>
              ) : (
                <div key={i} className="aph-turn aph-turn--assistant">
                  <div className="aph-assistant__avatar">
                    <MdIcon>auto_awesome</MdIcon>
                  </div>
                  <div className="aph-assistant__body">
                    {turn.pending && !turn.text ? (
                      <div className="aph-shimmer" aria-label="aphrody réfléchit">
                        <span></span>
                        <span></span>
                        <span></span>
                      </div>
                    ) : (
                      <div className={`aph-assistant__text${turn.pending ? " is-streaming" : ""}`}>
                        {turn.text}
                      </div>
                    )}
                  </div>
                </div>
              ),
            )}
          </div>
        </div>
      ) : (
        <div className="aph-assistant__hero">
          <h1 className="aph-assistant__hero-title">Par où commencer ?</h1>
        </div>
      )}

      <div className={`aph-composer-wrap${hasConversation ? " is-docked" : ""}`}>
        {hasConversation && (
          <MdIconButton
            className="aph-assistant__newchat"
            aria-label="Nouvelle conversation"
            title="Nouvelle conversation"
            onClick={newChat}
          >
            <MdIcon>add_comment</MdIcon>
          </MdIconButton>
        )}

        {attachments.length > 0 && (
          <div className="aph-chips" aria-label="Pièces jointes">
            {attachments.map((a) => (
              <div key={a.id} className={`aph-att-chip${a.loading ? " is-loading" : ""}`}>
                <MdIcon className="aph-att-chip__ico">{kindIcon(a.kind)}</MdIcon>
                <span className="aph-att-chip__meta">
                  <span className="aph-att-chip__name" title={a.name}>
                    {a.name}
                  </span>
                  {a.note ? (
                    <span className="aph-att-chip__note">{a.note}</span>
                  ) : (
                    a.detail && <span className="aph-att-chip__detail">{a.detail}</span>
                  )}
                </span>
                {a.loading ? (
                  <MdIcon className="aph-spin">progress_activity</MdIcon>
                ) : (
                  <MdIconButton
                    className="aph-att-chip__remove"
                    aria-label="Retirer"
                    onClick={() => removeAttachment(a.id)}
                  >
                    <MdIcon>close</MdIcon>
                  </MdIconButton>
                )}
              </div>
            ))}
          </div>
        )}

        {linkPanel && (
          <div className="aph-link-panel">
            <MdIcon>{kindIcon(linkPanel)}</MdIcon>
            <input
              className="aph-link-panel__input"
              autoFocus
              placeholder={linkPlaceholder()}
              value={linkValue}
              onChange={(e) => setLinkValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") submitLink();
                else if (e.key === "Escape") closeLinkPanel();
              }}
              aria-label="Adresse"
            />
            <MdIconButton aria-label="Ajouter" onClick={submitLink}>
              <MdIcon>add</MdIcon>
            </MdIconButton>
            <MdIconButton aria-label="Annuler" onClick={closeLinkPanel}>
              <MdIcon>close</MdIcon>
            </MdIconButton>
          </div>
        )}

        <div className="aph-composer">
          <Menu
            trigger={({ toggle }) => (
              <MdIconButton
                className="aph-composer__leading"
                title="Ajouter"
                aria-label="Ajouter une pièce jointe"
                onClick={toggle}
              >
                <MdIcon>add</MdIcon>
              </MdIconButton>
            )}
          >
            <MdMenuItem onClick={() => pickMedia("image")}>
              <MdIcon slot="start">image</MdIcon>
              <div slot="headline">Image</div>
            </MdMenuItem>
            <MdMenuItem onClick={() => pickMedia("video")}>
              <MdIcon slot="start">movie</MdIcon>
              <div slot="headline">Vidéo</div>
            </MdMenuItem>
            <MdMenuItem onClick={() => pickMedia("audio")}>
              <MdIcon slot="start">graphic_eq</MdIcon>
              <div slot="headline">Audio</div>
            </MdMenuItem>
            <MdMenuItem onClick={() => pickMedia("file")}>
              <MdIcon slot="start">description</MdIcon>
              <div slot="headline">Fichier (texte / code)</div>
            </MdMenuItem>
            <MdMenuItem onClick={() => openLinkPanel("github")}>
              <MdIcon slot="start">code</MdIcon>
              <div slot="headline">Lien GitHub</div>
            </MdMenuItem>
            <MdMenuItem onClick={() => openLinkPanel("url")}>
              <MdIcon slot="start">link</MdIcon>
              <div slot="headline">URL / Lien</div>
            </MdMenuItem>
            <MdMenuItem onClick={() => openLinkPanel("drive")}>
              <MdIcon slot="start">cloud</MdIcon>
              <div slot="headline">Google Drive</div>
            </MdMenuItem>
          </Menu>

          <input
            ref={fileInput}
            type="file"
            multiple
            className="aph-hidden-file"
            onChange={(e) => void onFilesChosen(e)}
            aria-hidden="true"
            tabIndex={-1}
          />

          <textarea
            ref={promptInput}
            className="aph-composer__input"
            rows={1}
            placeholder="Demandez à aphrody"
            value={draft}
            onChange={(e) => onInput(e.target.value)}
            onKeyDown={onKeydown}
            aria-label="Message"
          />

          <div className="aph-composer__trailing">
            <span className="aph-model-pill" title="Modèle de conversation">
              <span>{model.label}</span>
            </span>

            {speechSupported && !canSend && (
              <MdIconButton
                className="aph-mic-btn"
                onClick={() => setVoiceMode(true)}
                title="Mode vocal (conversation à voix haute)"
                aria-label="Mode vocal"
              >
                <MdIcon>graphic_eq</MdIcon>
              </MdIconButton>
            )}

            {canSend && (
              <MdIconButton
                className="aph-send-btn"
                onClick={() => void send()}
                disabled={busy}
                title="Envoyer"
                aria-label="Envoyer"
              >
                <MdIcon>arrow_upward</MdIcon>
              </MdIconButton>
            )}
          </div>
        </div>

        {!hasConversation && (
          <MdChipSet className="aph-suggestions" aria-label="Suggestions">
            {SUGGESTIONS.map((s) => (
              <MdAssistChip key={s.label} label={s.label} onClick={() => useSuggestion(s)}>
                <MdIcon slot="icon">{s.icon}</MdIcon>
              </MdAssistChip>
            ))}
          </MdChipSet>
        )}

        <p className="aph-disclaimer">
          aphrody peut afficher des informations inexactes. Vérifiez ses réponses.
        </p>
      </div>

      <VoiceOverlay open={voiceMode} onClose={() => setVoiceMode(false)} />
    </div>
  );
}
