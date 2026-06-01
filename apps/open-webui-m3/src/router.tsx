// TanStack Router tree (code-based). A pathless `_app` layout hosts the shell and
// guards auth (redirect to /auth when there's no token); /auth sits outside it.

import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { AppShell } from "./components/AppShell.tsx";
import { AuthScreen } from "./components/auth/AuthScreen.tsx";
import { SettingsDialog } from "./components/settings/SettingsDialog.tsx";
import { Home } from "./routes/Home.tsx";
import { ChatRoute } from "./routes/ChatRoute.tsx";
import { WorkspaceRoute } from "./routes/WorkspaceRoute.tsx";
import { AdminRoute } from "./routes/AdminRoute.tsx";
import { NotesRoute } from "./routes/NotesRoute.tsx";
import { auth } from "./api/client.ts";

function Root() {
  return (
    <>
      <Outlet />
      <SettingsDialog />
    </>
  );
}

const rootRoute = createRootRoute({ component: Root });

const authRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/auth",
  component: AuthScreen,
});

const appRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "_app",
  component: AppShell,
  beforeLoad: () => {
    if (!auth.token) throw redirect({ to: "/auth" });
  },
});

const indexRoute = createRoute({ getParentRoute: () => appRoute, path: "/", component: Home });
const chatRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/c/$chatId",
  component: ChatRoute,
});
const workspaceRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/workspace",
  component: WorkspaceRoute,
});
const adminRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/admin",
  component: AdminRoute,
});
const notesRoute = createRoute({
  getParentRoute: () => appRoute,
  path: "/notes",
  component: NotesRoute,
});

const routeTree = rootRoute.addChildren([
  authRoute,
  appRoute.addChildren([indexRoute, chatRoute, workspaceRoute, adminRoute, notesRoute]),
]);

export const router = createRouter({ routeTree, defaultPreload: "intent" });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
