// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

import { useState } from "react";
import { useParams } from "@tanstack/react-router";
import {
  MdFilledButton,
  MdIcon,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";
import {
  useAccount,
  useExec,
  useMeta,
  useRun,
  execText,
} from "./client.ts";
import {
  Panel,
  PageHead,
  CodeOutput,
  StatTile,
  Spinner,
} from "./ui.tsx";

export function AphrodySection() {
  const { section } = useParams({ from: "/_aphrody/a/$section" });
  const { data: meta, isLoading: loadingMeta } = useMeta();
  const { data: account } = useAccount();

  switch (section) {
    case "dashboard":
      return <DashboardView meta={meta} account={account} />;
    case "assistant":
      return <AssistantView />;
    case "skills":
      return <SkillsView />;
    case "mcp":
      return <McpView />;
    case "commands":
      return <CommandsView />;
    case "reverse":
      return <ReverseView />;
    case "forensics":
      return <ForensicsView />;
    case "network":
      return <NetworkView />;
    case "diagnostic":
      return <DiagnosticView />;
    case "settings":
      return <SettingsView meta={meta} />;
    case "about":
      return <AboutView account={account} />;
    default:
      return (
        <div style={{ padding: 24 }}>
          <PageHead title="Section introuvable" />
          <p className="aph-muted">La section requestée n'est pas implémentée.</p>
        </div>
      );
  }
}

function DashboardView({ meta, account }: { meta: any; account: any }) {
  const { data: mcpRes } = useExec(["mcp"], ["mcp"]);
  const { data: skillsRes } = useExec(["skills"], ["skills"]);

  const mcpCount = mcpRes?.stdout ? mcpRes.stdout.split("\n").length - 1 : 0;
  const skillsCount = skillsRes?.stdout ? skillsRes.stdout.split("\n").length - 1 : 0;

  return (
    <div className="aph-section">
      <PageHead
        title="Tableau de bord"
        subtitle="Vue d'ensemble de votre agent et environnement de RAG"
      />
      <div className="aph-grid">
        <StatTile
          icon="computer"
          label={`OS: ${meta?.target_os || "Linux"} (${meta?.target_arch || "x86_64"})`}
          value="Système Hôte"
          tone="var(--md-sys-color-primary)"
        />
        <StatTile
          icon="person"
          label={account?.email || "Non connecté"}
          value="Utilisateur"
          tone="var(--md-sys-color-tertiary)"
        />
        <StatTile
          icon="hub"
          label={`${mcpCount} serveur(s) configuré(s)`}
          value="Serveurs MCP"
          tone="var(--md-sys-color-error)"
        />
        <StatTile
          icon="extension"
          label={`${skillsCount} skill(s) disponible(s)`}
          value="Skills Actifs"
          tone="var(--md-sys-color-secondary)"
        />
      </div>

      <div style={{ marginTop: 24 }}>
        <Panel title="Statut de l'environnement" icon="stethoscope">
          <DiagnosticView showHead={false} />
        </Panel>
      </div>
    </div>
  );
}

function AssistantView() {
  const [messages, setMessages] = useState<Array<{ role: "user" | "model"; text: string }>>([
    { role: "model", text: "Bonjour ! Comment puis-je vous aider aujourd'hui ?" },
  ]);
  const [input, setInput] = useState("");
  const runMutation = useRun();

  const handleSend = async () => {
    if (!input.trim() || runMutation.isPending) return;
    const userText = input;
    setInput("");
    setMessages((prev) => [...prev, { role: "user", text: userText }]);

    const res = await runMutation.mutateAsync(["chat", userText]);
    const reply = execText(res).replace(/^aphrody · Gemini 3.5 Flash\n\n> .*\n\n/, "");
    setMessages((prev) => [...prev, { role: "model", text: reply }]);
  };

  return (
    <div className="aph-section" style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 48px)" }}>
      <PageHead title="Assistant aphrody" subtitle="Tour d'interaction avec le modèle Gemini" />
      <div style={{ flex: 1, overflowY: "auto", padding: "16px 0", display: "flex", flexDirection: "column", gap: 12 }}>
        {messages.map((msg, i) => (
          <div
            key={i}
            style={{
              alignSelf: msg.role === "user" ? "flex-end" : "flex-start",
              background: msg.role === "user" ? "var(--md-sys-color-primary-container)" : "var(--md-sys-color-surface-container)",
              color: msg.role === "user" ? "var(--md-sys-color-on-primary-container)" : "var(--md-sys-color-on-surface)",
              padding: "12px 16px",
              borderRadius: 16,
              maxWidth: "70%",
              whiteSpace: "pre-wrap",
            }}
          >
            {msg.text}
          </div>
        ))}
        {runMutation.isPending && (
          <div style={{ alignSelf: "flex-start" }}>
            <Spinner label="Gemini réfléchit..." />
          </div>
        )}
      </div>
      <div style={{ display: "flex", gap: 8, padding: "16px 0" }}>
        <MdOutlinedTextField
          label="Posez une question..."
          value={input}
          style={{ flex: 1 }}
          onInput={(e: any) => setInput(e.target.value)}
          onKeyDown={(e: any) => {
            if (e.key === "Enter") handleSend();
          }}
        />
        <MdFilledButton onClick={handleSend} disabled={runMutation.isPending}>
          <MdIcon slot="icon">send</MdIcon>
          Envoyer
        </MdFilledButton>
      </div>
    </div>
  );
}

function SkillsView() {
  const { data: res, isLoading } = useExec(["skills"], ["skills"]);

  return (
    <div className="aph-section">
      <PageHead title="Skills Agents" subtitle="Compétences étendues chargées par aphrody" />
      {isLoading ? (
        <Spinner label="Chargement des skills..." />
      ) : (
        <Panel title="Skills Installés" icon="extension">
          <CodeOutput text={execText(res)} />
        </Panel>
      )}
    </div>
  );
}

function McpView() {
  const { data: res, isLoading } = useExec(["mcp"], ["mcp"]);

  return (
    <div className="aph-section">
      <PageHead title="Serveurs MCP" subtitle="Protocoles Model Context Protocol connectés" />
      {isLoading ? (
        <Spinner label="Chargement des serveurs..." />
      ) : (
        <Panel title="Serveurs Actifs" icon="hub">
          <CodeOutput text={execText(res)} />
        </Panel>
      )}
    </div>
  );
}

function CommandsView() {
  const [argsStr, setArgsStr] = useState("");
  const [output, setOutput] = useState("");
  const runMutation = useRun();

  const handleRun = async () => {
    const args = argsStr.split(/\s+/).filter(Boolean);
    if (!args.length) return;
    const res = await runMutation.mutateAsync(args);
    setOutput(execText(res));
  };

  return (
    <div className="aph-section">
      <PageHead title="Console de commandes" subtitle="Exécuter des commandes directes sur le binaire aphrody" />
      <Panel title="Lancer une commande" icon="terminal">
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <MdOutlinedTextField
            label="Arguments (ex: version, mcp, re triage)"
            value={argsStr}
            style={{ flex: 1 }}
            onInput={(e: any) => setArgsStr(e.target.value)}
          />
          <MdFilledButton onClick={handleRun} disabled={runMutation.isPending}>
            Run
          </MdFilledButton>
        </div>
        <CodeOutput text={output} empty="Exécutez une commande pour voir la sortie." />
      </Panel>
    </div>
  );
}

function ReverseView() {
  const [filePath, setFilePath] = useState("/usr/bin/ls");
  const [output, setOutput] = useState("");
  const runMutation = useRun();

  const runRe = async (sub: string) => {
    const res = await runMutation.mutateAsync(["re", sub, "--file", filePath]);
    setOutput(execText(res));
  };

  return (
    <div className="aph-section">
      <PageHead title="Reverse Engineering" subtitle="Analyse statique et forensics de fichiers binaires" />
      <Panel title="Analyse de binaire" icon="frame_inspect">
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <MdOutlinedTextField
            label="Chemin absolu du fichier"
            value={filePath}
            style={{ flex: 1 }}
            onInput={(e: any) => setFilePath(e.target.value)}
          />
        </div>
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <MdFilledButton onClick={() => runRe("triage")} disabled={runMutation.isPending}>
            Triage
          </MdFilledButton>
          <MdFilledButton onClick={() => runRe("sections")} disabled={runMutation.isPending}>
            Sections
          </MdFilledButton>
          <MdFilledButton onClick={() => runRe("strings")} disabled={runMutation.isPending}>
            Strings
          </MdFilledButton>
        </div>
        <CodeOutput text={output} empty="Sélectionnez une analyse pour afficher les résultats." />
      </Panel>
    </div>
  );
}

function ForensicsView() {
  const { data: res, isLoading } = useExec(["forensics"], ["forensics"]);

  return (
    <div className="aph-section">
      <PageHead title="Forensics local" subtitle="Inspection et classification de profils d'environnements" />
      {isLoading ? (
        <Spinner label="Analyse en cours..." />
      ) : (
        <Panel title="Résultats de l'inspection statique" icon="folder_data">
          <CodeOutput text={execText(res)} />
        </Panel>
      )}
    </div>
  );
}

function NetworkView() {
  const [host, setHost] = useState("google.com");
  const [output, setOutput] = useState("");
  const runMutation = useRun();

  const handleResolve = async () => {
    const res = await runMutation.mutateAsync(["dns", "--host", host]);
    setOutput(execText(res));
  };

  return (
    <div className="aph-section">
      <PageHead title="Reconnaissance réseau" subtitle="Collecte d'informations DNS et WHOIS" />
      <Panel title="DNS Lookup" icon="lan">
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <MdOutlinedTextField
            label="Nom d'hôte"
            value={host}
            style={{ flex: 1 }}
            onInput={(e: any) => setHost(e.target.value)}
          />
          <MdFilledButton onClick={handleResolve} disabled={runMutation.isPending}>
            Lookup
          </MdFilledButton>
        </div>
        <CodeOutput text={output} empty="Saisissez un domaine pour lancer la résolution DNS." />
      </Panel>
    </div>
  );
}

function DiagnosticView({ showHead = true }: { showHead?: boolean }) {
  const { data: res, isLoading } = useExec(["doctor"], ["doctor"]);

  return (
    <div className={showHead ? "aph-section" : ""}>
      {showHead && <PageHead title="Diagnostics" subtitle="Vérification de l'état de santé du système" />}
      {isLoading ? (
        <Spinner label="Exécution du diagnostic..." />
      ) : (
        <CodeOutput text={execText(res)} />
      )}
    </div>
  );
}

function SettingsView({ meta }: { meta: any }) {
  return (
    <div className="aph-section">
      <PageHead title="Paramètres" subtitle="Configuration globale de l'application" />
      <Panel title="Configuration système" icon="settings">
        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          <MdOutlinedTextField label="Version binaire" value={meta?.app_version || ""} readOnly />
          <MdOutlinedTextField label="Architecture cible" value={meta?.target_arch || ""} readOnly />
          <MdOutlinedTextField label="OS cible" value={meta?.target_os || ""} readOnly />
        </div>
      </Panel>
    </div>
  );
}

function AboutView({ account }: { account: any }) {
  return (
    <div className="aph-section">
      <PageHead title="À propos" subtitle="Profil de l'agent connecté" />
      <Panel title="Mon Profil" icon="person">
        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          <MdOutlinedTextField label="Nom complet" value={account?.name || ""} readOnly />
          <MdOutlinedTextField label="Adresse e-mail" value={account?.email || ""} readOnly />
          <MdOutlinedTextField label="Initials" value={account?.initials || ""} readOnly />
        </div>
      </Panel>
    </div>
  );
}
