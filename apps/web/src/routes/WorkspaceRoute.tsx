// "/workspace" — Models / Knowledge / Prompts / Tools, each a searchable M3 card
// grid with a create CTA. Mirrors open-webui's workspace hub.

import { useMemo, useState } from "react";
import {
  MdAssistChip,
  MdFilledTonalButton,
  MdIcon,
  MdIconButton,
  MdOutlinedCard,
  MdPrimaryTab,
  MdSearchBar,
  MdTabs,
  MdDialog,
  MdFilledButton,
  MdTextButton,
  MdOutlinedTextField,
  MdOutlinedSelect,
  MdSelectOption,
} from "@aphrody/m3-react";
import { Menu } from "../components/ui/Menu.tsx";
import { MdMenuItem } from "@aphrody/m3-react";
import {
  useKnowledge,
  usePrompts,
  useTools,
  useWorkspaceModels,
  useCreateWorkspaceModel,
  useDeleteWorkspaceModel,
  useCreateKnowledge,
  useDeleteKnowledge,
  useCreatePrompt,
  useDeletePrompt,
  useCreateTool,
  useDeleteTool,
} from "../api/queries.ts";
import { CloudFirebaseTab } from "../components/workspace/CloudFirebaseTab.tsx";

interface Card {
  id: string;
  title: string;
  subtitle: string;
  icon: string;
  tags?: string[];
  badge?: string;
}

function CardGrid({
  cards,
  createLabel,
  onCreate,
  onDelete,
}: {
  cards: Card[];
  createLabel: string;
  onCreate?: () => void;
  onDelete?: (id: string) => void;
}) {
  const [q, setQ] = useState("");
  const filtered = useMemo(
    () => cards.filter((c) => (c.title + c.subtitle).toLowerCase().includes(q.toLowerCase())),
    [cards, q],
  );

  return (
    <>
      <div className="owui-spread" style={{ margin: "12px 0" }}>
        <div style={{ flex: "1 1 320px", maxWidth: 360 }}>
          <MdSearchBar
            value={q}
            placeholder="Rechercher"
            onInput={(e) => setQ((e.target as HTMLInputElement).value)}
          />
        </div>
        <MdFilledTonalButton onClick={onCreate}>
          <MdIcon slot="icon">add</MdIcon>
          {createLabel}
        </MdFilledTonalButton>
      </div>

      <div className="owui-grid">
        {filtered.map((c) => (
          <MdOutlinedCard key={c.id} style={{ padding: 16 }}>
            <div className="owui-spread">
              <div className="owui-row">
                <MdIcon style={{ color: "var(--md-sys-color-primary)" }}>{c.icon}</MdIcon>
                <strong>{c.title}</strong>
              </div>
              <Menu
                trigger={({ toggle }) => (
                  <MdIconButton aria-label="Options" onClick={toggle}>
                    <MdIcon>more_vert</MdIcon>
                  </MdIconButton>
                )}
              >
                <MdMenuItem>
                  <MdIcon slot="start">edit</MdIcon>
                  <span slot="headline">Modifier</span>
                </MdMenuItem>
                <MdMenuItem>
                  <MdIcon slot="start">content_copy</MdIcon>
                  <span slot="headline">Cloner</span>
                </MdMenuItem>
                <MdMenuItem onClick={() => onDelete?.(c.id)}>
                  <MdIcon slot="start">delete</MdIcon>
                  <span slot="headline">Supprimer</span>
                </MdMenuItem>
              </Menu>
            </div>
            <p className="owui-muted" style={{ margin: "6px 0 10px", fontSize: 14 }}>
              {c.subtitle}
            </p>
            <div className="owui-row" style={{ flexWrap: "wrap" }}>
              {c.badge && <MdAssistChip label={c.badge} />}
              {c.tags?.map((t) => (
                <MdAssistChip key={t} label={t} />
              ))}
            </div>
          </MdOutlinedCard>
        ))}
      </div>
    </>
  );
}

export function WorkspaceRoute() {
  const [tab, setTab] = useState(0);
  const { data: models = [] } = useWorkspaceModels();
  const { data: knowledge = [] } = useKnowledge();
  const { data: prompts = [] } = usePrompts();
  const { data: tools = [] } = useTools();

  // Mutations
  const createModel = useCreateWorkspaceModel();
  const deleteModel = useDeleteWorkspaceModel();
  const createKnowledge = useCreateKnowledge();
  const deleteKnowledge = useDeleteKnowledge();
  const createPrompt = useCreatePrompt();
  const deletePrompt = useDeletePrompt();
  const createTool = useCreateTool();
  const deleteTool = useDeleteTool();

  // Model Form Dialog State
  const [modelOpen, setModelOpen] = useState(false);
  const [modelName, setModelName] = useState("");
  const [modelDesc, setModelDesc] = useState("");
  const [modelBase, setModelBase] = useState("gpt-4o");
  const [modelVisibility, setModelVisibility] = useState<"public" | "private">("public");
  const [modelTags, setModelTags] = useState("");

  const handleCreateModel = async () => {
    if (!modelName.trim()) return;
    try {
      await createModel.mutateAsync({
        name: modelName,
        description: modelDesc,
        base_model_id: modelBase,
        visibility: modelVisibility,
        tags: modelTags.split(",").map((t) => t.trim()).filter(Boolean),
      });
      setModelName("");
      setModelDesc("");
      setModelBase("gpt-4o");
      setModelVisibility("public");
      setModelTags("");
      setModelOpen(false);
    } catch (err) {
      console.error(err);
    }
  };

  // Knowledge Form Dialog State
  const [knowledgeOpen, setKnowledgeOpen] = useState(false);
  const [knowledgeName, setKnowledgeName] = useState("");
  const [knowledgeDesc, setKnowledgeDesc] = useState("");

  const handleCreateKnowledge = async () => {
    if (!knowledgeName.trim()) return;
    try {
      await createKnowledge.mutateAsync({
        name: knowledgeName,
        description: knowledgeDesc,
        file_count: 0,
      });
      setKnowledgeName("");
      setKnowledgeDesc("");
      setKnowledgeOpen(false);
    } catch (err) {
      console.error(err);
    }
  };

  // Prompt Form Dialog State
  const [promptOpen, setPromptOpen] = useState(false);
  const [promptTitle, setPromptTitle] = useState("");
  const [promptCommand, setPromptCommand] = useState("/");
  const [promptContent, setPromptContent] = useState("");
  const [promptTags, setPromptTags] = useState("");

  const handleCreatePrompt = async () => {
    if (!promptTitle.trim() || !promptCommand.trim() || !promptContent.trim()) return;
    let cmd = promptCommand.trim();
    if (!cmd.startsWith("/")) {
      cmd = "/" + cmd;
    }
    try {
      await createPrompt.mutateAsync({
        title: promptTitle,
        command: cmd,
        content: promptContent,
        tags: promptTags.split(",").map((t) => t.trim()).filter(Boolean),
      });
      setPromptTitle("");
      setPromptCommand("/");
      setPromptContent("");
      setPromptTags("");
      setPromptOpen(false);
    } catch (err) {
      console.error(err);
    }
  };

  // Tool Form Dialog State
  const [toolOpen, setToolOpen] = useState(false);
  const [toolName, setToolName] = useState("");
  const [toolDesc, setToolDesc] = useState("");
  const [toolType, setToolType] = useState<"custom" | "openapi">("custom");

  const handleCreateTool = async () => {
    if (!toolName.trim()) return;
    try {
      await createTool.mutateAsync({
        name: toolName,
        description: toolDesc,
        type: toolType,
      });
      setToolName("");
      setToolDesc("");
      setToolType("custom");
      setToolOpen(false);
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="owui-page">
      <div className="owui-page__inner">
        <h1 style={{ marginTop: 0 }}>Espace de travail</h1>
        <MdTabs
          activeTabIndex={tab}
          onChange={(e) =>
            setTab((e.target as unknown as { activeTabIndex: number }).activeTabIndex)
          }
        >
          <MdPrimaryTab>
            <MdIcon slot="icon">deployed_code</MdIcon>Modèles
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">menu_book</MdIcon>Documents
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">terminal</MdIcon>Prompts
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">build</MdIcon>Outils
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">cloud</MdIcon>Cloud & Firebase
          </MdPrimaryTab>
        </MdTabs>

        {tab === 0 && (
          <CardGrid
            createLabel="Créer un modèle"
            onCreate={() => setModelOpen(true)}
            onDelete={(id) => deleteModel.mutate(id)}
            cards={models.map((m) => ({
              id: m.id,
              title: m.name,
              subtitle: m.description,
              icon: "deployed_code",
              tags: m.tags,
              badge: m.visibility,
            }))}
          />
        )}
        {tab === 1 && (
          <CardGrid
            createLabel="Créer un document"
            onCreate={() => setKnowledgeOpen(true)}
            onDelete={(id) => deleteKnowledge.mutate(id)}
            cards={knowledge.map((k) => ({
              id: k.id,
              title: k.name,
              subtitle: k.description,
              icon: "menu_book",
              badge: `${k.file_count} fichiers`,
            }))}
          />
        )}
        {tab === 2 && (
          <CardGrid
            createLabel="Créer un prompt"
            onCreate={() => setPromptOpen(true)}
            onDelete={(id) => deletePrompt.mutate(id)}
            cards={prompts.map((p) => ({
              id: p.id,
              title: p.title,
              subtitle: p.content,
              icon: "terminal",
              tags: p.tags,
              badge: p.command,
            }))}
          />
        )}
        {tab === 3 && (
          <CardGrid
            createLabel="Créer un outil"
            onCreate={() => setToolOpen(true)}
            onDelete={(id) => deleteTool.mutate(id)}
            cards={tools.map((t) => ({
              id: t.id,
              title: t.name,
              subtitle: t.description,
              icon: "build",
              badge: t.type,
            }))}
          />
        )}
        {tab === 4 && <CloudFirebaseTab />}
      </div>

      {/* Model Creation Dialog */}
      <MdDialog open={modelOpen} onClosed={() => setModelOpen(false)}>
        <div slot="headline">Créer un modèle</div>
        <form slot="content" method="dialog" className="owui-stack" style={{ minWidth: 360 }}>
          <MdOutlinedTextField
            label="Nom du modèle"
            value={modelName}
            onInput={(e) => setModelName((e.target as HTMLInputElement).value)}
            required
          />
          <MdOutlinedTextField
            label="Description"
            value={modelDesc}
            onInput={(e) => setModelDesc((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedSelect
            label="Modèle de base"
            value={modelBase}
            onChange={(e) => setModelBase((e.target as HTMLSelectElement).value)}
          >
            <MdSelectOption value="gpt-4o">
              <span slot="headline">GPT-4o</span>
            </MdSelectOption>
            <MdSelectOption value="gpt-3.5-turbo">
              <span slot="headline">GPT-3.5 Turbo</span>
            </MdSelectOption>
            <MdSelectOption value="llama3">
              <span slot="headline">Llama 3</span>
            </MdSelectOption>
            <MdSelectOption value="gemini-1.5-pro">
              <span slot="headline">Gemini 1.5 Pro</span>
            </MdSelectOption>
          </MdOutlinedSelect>
          <MdOutlinedSelect
            label="Visibilité"
            value={modelVisibility}
            onChange={(e) =>
              setModelVisibility((e.target as HTMLSelectElement).value as "public" | "private")
            }
          >
            <MdSelectOption value="public">
              <span slot="headline">Public</span>
            </MdSelectOption>
            <MdSelectOption value="private">
              <span slot="headline">Privé</span>
            </MdSelectOption>
          </MdOutlinedSelect>
          <MdOutlinedTextField
            label="Tags (séparés par des virgules)"
            value={modelTags}
            onInput={(e) => setModelTags((e.target as HTMLInputElement).value)}
            placeholder="e.g. chat, codegen"
          />
        </form>
        <div slot="actions">
          <MdTextButton onClick={() => setModelOpen(false)}>Annuler</MdTextButton>
          <MdFilledButton onClick={handleCreateModel} disabled={!modelName.trim()}>
            Créer
          </MdFilledButton>
        </div>
      </MdDialog>

      {/* Knowledge Creation Dialog */}
      <MdDialog open={knowledgeOpen} onClosed={() => setKnowledgeOpen(false)}>
        <div slot="headline">Créer un document</div>
        <form slot="content" method="dialog" className="owui-stack" style={{ minWidth: 360 }}>
          <MdOutlinedTextField
            label="Nom du document"
            value={knowledgeName}
            onInput={(e) => setKnowledgeName((e.target as HTMLInputElement).value)}
            required
          />
          <MdOutlinedTextField
            label="Description"
            value={knowledgeDesc}
            onInput={(e) => setKnowledgeDesc((e.target as HTMLInputElement).value)}
          />
        </form>
        <div slot="actions">
          <MdTextButton onClick={() => setKnowledgeOpen(false)}>Annuler</MdTextButton>
          <MdFilledButton onClick={handleCreateKnowledge} disabled={!knowledgeName.trim()}>
            Créer
          </MdFilledButton>
        </div>
      </MdDialog>

      {/* Prompt Creation Dialog */}
      <MdDialog open={promptOpen} onClosed={() => setPromptOpen(false)}>
        <div slot="headline">Créer un prompt</div>
        <form slot="content" method="dialog" className="owui-stack" style={{ minWidth: 360 }}>
          <MdOutlinedTextField
            label="Titre"
            value={promptTitle}
            onInput={(e) => setPromptTitle((e.target as HTMLInputElement).value)}
            required
          />
          <MdOutlinedTextField
            label="Commande (ex: /code)"
            value={promptCommand}
            onInput={(e) => setPromptCommand((e.target as HTMLInputElement).value)}
            required
          />
          <MdOutlinedTextField
            type="textarea"
            rows={4}
            label="Contenu du prompt"
            value={promptContent}
            onInput={(e) => setPromptContent((e.target as HTMLInputElement).value)}
            required
          />
          <MdOutlinedTextField
            label="Tags (séparés par des virgules)"
            value={promptTags}
            onInput={(e) => setPromptTags((e.target as HTMLInputElement).value)}
            placeholder="e.g. system, code"
          />
        </form>
        <div slot="actions">
          <MdTextButton onClick={() => setPromptOpen(false)}>Annuler</MdTextButton>
          <MdFilledButton
            onClick={handleCreatePrompt}
            disabled={!promptTitle.trim() || !promptCommand.trim() || !promptContent.trim()}
          >
            Créer
          </MdFilledButton>
        </div>
      </MdDialog>

      {/* Tool Creation Dialog */}
      <MdDialog open={toolOpen} onClosed={() => setToolOpen(false)}>
        <div slot="headline">Créer un outil</div>
        <form slot="content" method="dialog" className="owui-stack" style={{ minWidth: 360 }}>
          <MdOutlinedTextField
            label="Nom de l'outil"
            value={toolName}
            onInput={(e) => setToolName((e.target as HTMLInputElement).value)}
            required
          />
          <MdOutlinedTextField
            label="Description"
            value={toolDesc}
            onInput={(e) => setToolDesc((e.target as HTMLInputElement).value)}
          />
          <MdOutlinedSelect
            label="Type"
            value={toolType}
            onChange={(e) => setToolType((e.target as HTMLSelectElement).value as "custom" | "openapi")}
          >
            <MdSelectOption value="custom">
              <span slot="headline">Personnalisé</span>
            </MdSelectOption>
            <MdSelectOption value="openapi">
              <span slot="headline">OpenAPI</span>
            </MdSelectOption>
          </MdOutlinedSelect>
        </form>
        <div slot="actions">
          <MdTextButton onClick={() => setToolOpen(false)}>Annuler</MdTextButton>
          <MdFilledButton onClick={handleCreateTool} disabled={!toolName.trim()}>
            Créer
          </MdFilledButton>
        </div>
      </MdDialog>
    </div>
  );
}
