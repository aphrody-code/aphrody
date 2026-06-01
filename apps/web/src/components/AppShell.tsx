// In-app layout: M3 top app bar + collapsible sidebar (CSS grid) wrapping the
// routed page (<Outlet/>). Auth is guarded in the router (redirect to /auth).

import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import { MdIcon, MdIconButton, MdMenuItem, MdTopAppBar } from "@aphrody-code/m3-react";
import { Menu } from "./ui/Menu.tsx";
import { Sidebar } from "./Sidebar.tsx";
import { useConfig } from "../api/queries.ts";
import { getState, session, setState, useUi } from "../store.ts";

function titleFor(path: string): string {
  if (path === "/" || path.startsWith("/c/")) return "Chat";
  if (path.startsWith("/workspace")) return "Workspace";
  if (path.startsWith("/admin")) return "Admin";
  if (path.startsWith("/notes")) return "Notes";
  return "Open WebUI";
}

export function AppShell() {
  const { sidebarCollapsed, themeMode, user } = useUi();
  const { data: config } = useConfig();
  const navigate = useNavigate();
  const path = useRouterState({ select: (s) => s.location.pathname });

  const cycleTheme = () => {
    const order = ["system", "light", "dark"] as const;
    const next = order[(order.indexOf(getState().themeMode) + 1) % order.length];
    setState({ themeMode: next });
  };
  const themeIcon =
    themeMode === "dark" ? "dark_mode" : themeMode === "light" ? "light_mode" : "brightness_auto";

  return (
    <div className="owui-shell" data-collapsed={sidebarCollapsed}>
      {!sidebarCollapsed && (
        <aside className="owui-sidebar">
          <Sidebar />
        </aside>
      )}

      <div className="owui-main">
        <MdTopAppBar variant="small">
          <MdIconButton
            slot="leading"
            aria-label="Toggle sidebar"
            onClick={() => setState({ sidebarCollapsed: !sidebarCollapsed })}
          >
            <MdIcon>{sidebarCollapsed ? "menu" : "menu_open"}</MdIcon>
          </MdIconButton>

          <span>{config?.name ?? titleFor(path)}</span>

          <MdIconButton slot="trailing" aria-label="Toggle theme" onClick={cycleTheme}>
            <MdIcon>{themeIcon}</MdIcon>
          </MdIconButton>
          <MdIconButton
            slot="trailing"
            aria-label="Settings"
            onClick={() => setState({ settingsOpen: true })}
          >
            <MdIcon>settings</MdIcon>
          </MdIconButton>

          <span slot="trailing">
            <Menu
              trigger={({ toggle }) => (
                <MdIconButton aria-label="Account" onClick={toggle}>
                  <MdIcon>account_circle</MdIcon>
                </MdIconButton>
              )}
            >
              <MdMenuItem disabled>
                <span slot="headline">{user?.name ?? "Account"}</span>
                <span slot="supporting-text">{user?.email}</span>
              </MdMenuItem>
              <MdMenuItem onClick={() => setState({ settingsOpen: true })}>
                <MdIcon slot="start">tune</MdIcon>
                <span slot="headline">Settings</span>
              </MdMenuItem>
              {config?.features.enable_admin_panel && (
                <MdMenuItem onClick={() => void navigate({ to: "/admin" })}>
                  <MdIcon slot="start">admin_panel_settings</MdIcon>
                  <span slot="headline">Admin Panel</span>
                </MdMenuItem>
              )}
              <MdMenuItem
                onClick={() => {
                  session.signOut();
                  void navigate({ to: "/auth" });
                }}
              >
                <MdIcon slot="start">logout</MdIcon>
                <span slot="headline">Sign out</span>
              </MdMenuItem>
            </Menu>
          </span>
        </MdTopAppBar>

        <Outlet />
      </div>
    </div>
  );
}
