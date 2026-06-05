// App shell for the ported aphrody desktop: the narrow (~76px) icon rail on the
// deep glow background + the routed content. Port of Angular AppComponent's rail.

import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import { MdIcon, MdIconButton } from "@aphrody/m3-react";
import { useAccount } from "./client.ts";
import { getState, setState, useUi } from "../store.ts";

interface NavItem {
  seg: string;
  icon: string;
  label: string;
}

const MAIN: NavItem[] = [
  { seg: "assistant", icon: "chat_bubble", label: "Assistant" },
  { seg: "dashboard", icon: "dashboard", label: "Accueil" },
  { seg: "studio", icon: "movie", label: "Studio" },
  { seg: "skills", icon: "extension", label: "Skills" },
  { seg: "mcp", icon: "hub", label: "MCP" },
  { seg: "commands", icon: "apps", label: "Commandes" },
];

const TOOLS: NavItem[] = [
  { seg: "reverse", icon: "frame_inspect", label: "Reverse" },
  { seg: "forensics", icon: "folder_data", label: "Forensics" },
  { seg: "network", icon: "lan", label: "Réseau" },
  { seg: "diagnostic", icon: "stethoscope", label: "Diagnostic" },
];

function RailButton({
  item,
  active,
  onClick,
}: {
  item: NavItem;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <MdIconButton
      className={`aph-rail__btn${active ? " is-active" : ""}`}
      aria-label={item.label}
      title={item.label}
      onClick={onClick}
    >
      <MdIcon>{item.icon}</MdIcon>
    </MdIconButton>
  );
}

export function AphrodyShell() {
  const navigate = useNavigate();
  const path = useRouterState({ select: (s) => s.location.pathname });
  const { themeMode } = useUi();
  const { data: account } = useAccount();

  const go = (seg: string) => void navigate({ to: "/a/$section", params: { section: seg } });
  const isActive = (seg: string) => path === `/a/${seg}`;

  const toggleTheme = () => {
    const next = getState().themeMode === "light" ? "dark" : "light";
    setState({ themeMode: next });
  };

  return (
    <div className="aph-shell">
      <aside className="aph-rail">
        <div className="aph-rail__brand" title="aphrody">
          <MdIcon>auto_awesome</MdIcon>
        </div>

        <nav className="aph-rail__group">
          {MAIN.map((it) => (
            <RailButton
              key={it.seg}
              item={it}
              active={isActive(it.seg)}
              onClick={() => go(it.seg)}
            />
          ))}
        </nav>

        <div className="aph-rail__divider" />

        <nav className="aph-rail__group">
          {TOOLS.map((it) => (
            <RailButton
              key={it.seg}
              item={it}
              active={isActive(it.seg)}
              onClick={() => go(it.seg)}
            />
          ))}
        </nav>

        <div className="aph-rail__spacer" />

        <div className="aph-rail__group">
          <MdIconButton aria-label="Thème" title="Thème" onClick={toggleTheme}>
            <MdIcon>{themeMode === "light" ? "light_mode" : "dark_mode"}</MdIcon>
          </MdIconButton>
          <button
            className="aph-rail__avatar"
            title={account?.connected ? account.email : "Compte"}
            onClick={() => go("about")}
          >
            {account?.connected ? account.initials : <MdIcon>person</MdIcon>}
          </button>
          <RailButton
            item={{ seg: "settings", icon: "settings", label: "Paramètres" }}
            active={isActive("settings")}
            onClick={() => go("settings")}
          />
        </div>
      </aside>

      <main className="aph-content">
        <Outlet />
      </main>
    </div>
  );
}
