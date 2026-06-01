// Section router for the ported aphrody admin app: maps the /a/$section param to
// the real ported feature component. One switch, eleven faithful screens.

import { useParams } from "@tanstack/react-router";
import { PageHead } from "./ui.tsx";
import { Assistant } from "./features/assistant/Assistant.tsx";
import { Dashboard } from "./features/dashboard/Dashboard.tsx";
import { Skills } from "./features/skills/Skills.tsx";
import { Mcp } from "./features/mcp/Mcp.tsx";
import { Commands } from "./features/commands/Commands.tsx";
import { Reverse } from "./features/reverse/Reverse.tsx";
import { Forensics } from "./features/forensics/Forensics.tsx";
import { Network } from "./features/network/Network.tsx";
import { Diagnostic } from "./features/diagnostic/Diagnostic.tsx";
import { Settings } from "./features/settings/Settings.tsx";
import { About } from "./features/about/About.tsx";

export function AphrodySection() {
  const { section } = useParams({ from: "/_aphrody/a/$section" });

  switch (section) {
    case "assistant":
      return <Assistant />;
    case "dashboard":
      return <Dashboard />;
    case "skills":
      return <Skills />;
    case "mcp":
      return <Mcp />;
    case "commands":
      return <Commands />;
    case "reverse":
      return <Reverse />;
    case "forensics":
      return <Forensics />;
    case "network":
      return <Network />;
    case "diagnostic":
      return <Diagnostic />;
    case "settings":
      return <Settings />;
    case "about":
      return <About />;
    default:
      return (
        <div style={{ padding: 24 }}>
          <PageHead title="Section introuvable" subtitle={`« ${section} » n'existe pas.`} />
        </div>
      );
  }
}
