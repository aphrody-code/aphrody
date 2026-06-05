// React entry: load the Material Symbols font, wire QueryClient + Theme + Router,
// and restore any persisted session before first paint.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { ensureMaterialSymbols } from "@aphrody/material-web/icon/material-symbols.js";
import "@aphrody/m3-tokens/m3-tokens.css";
import "./theme.css";
import "./app.css";
import "./aphrody/aphrody.css";
import { router } from "./router.tsx";
import { ThemeProvider } from "./theme/ThemeProvider.tsx";
import { api, auth } from "./api/client.ts";
import { setState } from "./store.ts";

// Load the variable Material Symbols font (all axis ranges) so md-icon renders.
ensureMaterialSymbols();

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});

// Restore session from a persisted token (store.user isn't persisted).
async function boot() {
  // Extract token from URL query string if present
  const params = new URLSearchParams(window.location.search);
  const urlToken = params.get("token");
  if (urlToken && urlToken.length === 16) {
    auth.set(urlToken);
    // Remove token from URL for security
    const cleanUrl = window.location.pathname + window.location.hash;
    window.history.replaceState({}, document.title, cleanUrl);
  }

  if (auth.token) {
    try {
      const user = await api.getSession();
      setState({ user });
    } catch {
      auth.clear();
    }
  }

  const root = document.getElementById("root");
  if (!root) throw new Error("#root missing");
  createRoot(root).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider>
          <RouterProvider router={router} />
        </ThemeProvider>
      </QueryClientProvider>
    </StrictMode>,
  );
}

void boot();
