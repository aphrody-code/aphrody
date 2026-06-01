// Minimal global UI store (theme, seed, sidebar, session, modals) built on
// useSyncExternalStore — no extra dependency, stays inside the Bun + TanStack rule.

import { useSyncExternalStore } from "react";
import { auth } from "./api/client.ts";
import type { SessionUser } from "./api/types.ts";

export type ThemeMode = "light" | "dark" | "system";

export interface UiState {
  themeMode: ThemeMode;
  seed: string;
  sidebarCollapsed: boolean;
  settingsOpen: boolean;
  user: SessionUser | null;
}

const STORAGE = "owui-m3-ui";

function load(): UiState {
  const base: UiState = {
    themeMode: "system",
    seed: "#6750A4",
    sidebarCollapsed: false,
    settingsOpen: false,
    user: null,
  };
  try {
    const raw = localStorage.getItem(STORAGE);
    if (raw) Object.assign(base, JSON.parse(raw));
  } catch {
    /* first run */
  }
  return base;
}

let state: UiState = load();
const listeners = new Set<() => void>();

function persist() {
  const { user: _user, settingsOpen: _s, ...rest } = state;
  try {
    localStorage.setItem(STORAGE, JSON.stringify(rest));
  } catch {
    /* quota / private mode */
  }
}

export function setState(patch: Partial<UiState>) {
  state = { ...state, ...patch };
  persist();
  listeners.forEach((l) => l());
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

export function useUi(): UiState {
  return useSyncExternalStore(
    subscribe,
    () => state,
    () => state,
  );
}

export function getState(): UiState {
  return state;
}

/** Resolve "system" against the OS preference. */
export function isDark(mode: ThemeMode): boolean {
  if (mode === "dark") return true;
  if (mode === "light") return false;
  return typeof matchMedia !== "undefined" && matchMedia("(prefers-color-scheme: dark)").matches;
}

export const session = {
  signIn(user: SessionUser) {
    auth.set(user.token);
    setState({ user });
  },
  signOut() {
    auth.clear();
    setState({ user: null });
  },
};
