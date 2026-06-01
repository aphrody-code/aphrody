// aphrody CLI bridge for the web port. The Angular AphrodyService ran commands
// through the Tauri `aphrody_exec` invoke, with a `POST /api/run` browser
// fallback. On the web we always use that fallback — the Bun mock server returns
// realistic ExecResults so every screen is exercisable without a backend.

import { useMutation, useQuery } from "@tanstack/react-query";
import type { Account, ExecResult, Meta } from "./types.ts";

/** Run an aphrody command. `args` is argv WITHOUT the program name. */
export async function run(args: string[]): Promise<ExecResult> {
  try {
    const res = await fetch("/api/run", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ args }),
    });
    if (res.ok) return (await res.json()) as ExecResult;
    return { code: res.status, stdout: "", stderr: `HTTP ${res.status}` };
  } catch (e) {
    return { code: -1, stdout: "", stderr: e instanceof Error ? e.message : "network error" };
  }
}

/** stdout when the run succeeded, otherwise stderr — the text to show. */
export function execText(r: ExecResult | undefined): string {
  if (!r) return "";
  return r.code === 0 ? r.stdout : r.stderr || r.stdout;
}

/** Best-effort JSON parse of stdout (commands that emit `--json`). */
export function execJson<T>(r: ExecResult | undefined): T | null {
  if (!r?.stdout) return null;
  try {
    return JSON.parse(r.stdout) as T;
  } catch {
    const m = r.stdout.match(/[[{][\s\S]*[\]}]/);
    if (m) {
      try {
        return JSON.parse(m[0]) as T;
      } catch {
        return null;
      }
    }
    return null;
  }
}

/** Imperative run as a TanStack mutation (button-triggered commands). */
export function useRun() {
  return useMutation({ mutationFn: (args: string[]) => run(args) });
}

/** Run-on-mount command (panels that load data). `key` namespaces the cache. */
export function useExec(key: readonly unknown[], args: string[], enabled = true) {
  return useQuery({
    queryKey: ["aphrody", ...key],
    queryFn: () => run(args),
    enabled,
    staleTime: 30_000,
  });
}

export function useMeta() {
  return useQuery({
    queryKey: ["aphrody", "meta"],
    queryFn: async () => {
      const res = await fetch("/api/meta");
      return (await res.json()) as Meta;
    },
    staleTime: Infinity,
  });
}

export function useAccount() {
  return useQuery({
    queryKey: ["aphrody", "account"],
    queryFn: async () => {
      const res = await fetch("/api/account");
      return (await res.json()) as Account;
    },
  });
}
