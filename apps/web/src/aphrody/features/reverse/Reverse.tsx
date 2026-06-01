// Reverse-engineering view. The triage pillar (R5) renders through the typed
// TriagePanel; the remaining `aphrody re` sub-commands keep the generic raw
// ToolRunner.

import { PageHead } from "../../ui.tsx";
import { ToolRunner, type ToolAction } from "../../ToolRunner.tsx";
import { TriagePanel } from "./TriagePanel.tsx";

const ACTIONS: ToolAction[] = [
  {
    label: "Extraire les chaînes",
    icon: "data_array",
    args: ["re", "strings"],
    prompt: { placeholder: "Chemin du binaire" },
    hint: "Chaînes ASCII et UTF-16LE imprimables.",
  },
  {
    label: "Lister les sections",
    icon: "view_module",
    args: ["re", "sections"],
    prompt: { placeholder: "Chemin du binaire" },
    hint: "Table des sections avec entropie de Shannon.",
  },
  {
    label: "Endpoints Google",
    icon: "travel_explore",
    args: ["re", "google"],
    prompt: { placeholder: "Chemin du binaire" },
    hint: "Extraction des endpoints/URLs Google embarqués.",
  },
  {
    label: "Détection Go",
    icon: "code",
    args: ["re", "go"],
    prompt: { placeholder: "Chemin du binaire" },
    hint: "Détecte un binaire Go et son buildinfo.",
  },
  {
    label: "Analyse automatique complète",
    icon: "auto_awesome",
    args: ["re", "auto"],
    prompt: { placeholder: "Chemin du binaire ou dossier" },
    hint: "Enchaîne triage + strings + endpoints + Go + désassemblage.",
  },
];

export function Reverse() {
  return (
    <div className="aph-section aph-reverse">
      <PageHead
        title="Reverse engineering"
        subtitle="Triage et analyse de binaires PE / ELF (aphrody re)."
      />

      <TriagePanel />

      <div className="aph-reverse__more">
        <h2 className="aph-subtitle">Autres analyses</h2>
        <ToolRunner
          title="Outils complémentaires"
          subtitle="Chaînes, sections brutes, endpoints Google, détection Go, analyse complète."
          icon="construction"
          actions={ACTIONS}
        />
      </div>
    </div>
  );
}
