/** @license SPDX-License-Identifier: Apache-2.0 */

// Top-level render() entry point. Wires React → reconciler → OSC sink and
// installs the input/window-size/focus listeners that feed the hook contexts.

import { ConcurrentRoot } from "react-reconciler/constants.js";
import { createElement, type ReactNode } from "react";
import { aphrodyReconciler, createRoot, disposeRoot, type ReconcilerRoot } from "./reconciler.ts";
import { OscFrameParser, type OscFrame, type OscSink } from "./osc.ts";
import { AppContext, type AppContextValue } from "./hooks/use-app.ts";
import { StdoutContext } from "./hooks/use-stdout.ts";
import { InputContext, emptyKey, type InputContextValue } from "./hooks/use-input.ts";
import {
  WindowSizeContext,
  type WindowSizeContextValue,
} from "./hooks/use-window-size.ts";
import { FocusContext, type FocusContextValue } from "./hooks/use-focus.ts";
import type { InputHandler, InputKey, RenderOptions, WindowSize } from "./types.ts";

export interface Instance {
  rerender(element: ReactNode): void;
  unmount(): void;
  waitUntilExit(): Promise<void>;
  cleanup(): void;
}

// Default WebSocket endpoint for the `ws` target. Matches the aphrody A2A
// listener port documented in CLAUDE.md §6.1.
const DEFAULT_WS_URL = "ws://127.0.0.1:8788/aphrody-jsx";

// Resolve which target to use when "auto" is requested.
function resolveTarget(options: RenderOptions): "pty" | "ws" {
  if (options.target && options.target !== "auto") return options.target;
  if (typeof process !== "undefined" && (process.stdout as { isTTY?: boolean })?.isTTY) {
    return "pty";
  }
  return "ws";
}

// Wraps a Node-style stdout into an OscSink, normalizing string vs Uint8Array.
function ptySink(): OscSink {
  // Bun exposes process.stdout with both write(string) and write(Uint8Array).
  // We forward both forms unchanged.
  return {
    write(chunk: Uint8Array | string): void {
      process.stdout.write(chunk as never);
    },
  };
}

// WebSocket sink. Buffers writes until the socket is open, then flushes.
function wsSink(url: string): { sink: OscSink; close(): void } {
  const pending: (Uint8Array | string)[] = [];
  let ready = false;
  const WS: typeof WebSocket | undefined =
    typeof WebSocket !== "undefined" ? WebSocket : undefined;
  if (WS === undefined) {
    throw new Error("@aphrody/jsx: WebSocket is not available in this runtime");
  }
  const socket = new WS(url);
  socket.binaryType = "arraybuffer";
  socket.addEventListener("open", () => {
    ready = true;
    for (const chunk of pending) socket.send(chunk as never);
    pending.length = 0;
  });
  return {
    sink: {
      write(chunk: Uint8Array | string): void {
        if (ready) socket.send(chunk as never);
        else pending.push(chunk);
      },
    },
    close(): void {
      try {
        socket.close();
      } catch {
        /* already closed */
      }
    },
  };
}

// Build the InputContextValue + the dispatcher used by the OSC frame parser.
function makeInputContext(): {
  ctx: InputContextValue;
  dispatch(input: string, key: InputKey): void;
} {
  const subs = new Set<InputHandler>();
  const ctx: InputContextValue = {
    isRawModeSupported:
      typeof process !== "undefined" && Boolean((process.stdin as { isTTY?: boolean })?.isTTY),
    subscribe(handler): () => void {
      subs.add(handler);
      return () => {
        subs.delete(handler);
      };
    },
  };
  function dispatch(input: string, key: InputKey): void {
    for (const handler of [...subs]) {
      try {
        handler(input, key);
      } catch (err) {
        reportError(err);
      }
    }
  }
  return { ctx, dispatch };
}

// Build the WindowSizeContextValue + a setter used by the frame dispatcher.
function makeWindowSizeContext(initial: WindowSize): {
  ctx: WindowSizeContextValue;
  set(size: WindowSize): void;
} {
  const subs = new Set<(size: WindowSize) => void>();
  const ctx: WindowSizeContextValue = {
    current: initial,
    subscribe(listener): () => void {
      subs.add(listener);
      return () => {
        subs.delete(listener);
      };
    },
  };
  function set(size: WindowSize): void {
    ctx.current = size;
    for (const listener of [...subs]) listener(size);
  }
  return { ctx, set };
}

// Build the FocusContextValue. Focus is a small ordered list with an active
// index; subscribers are notified on every change.
function makeFocusContext(): {
  ctx: FocusContextValue;
  setFocus(id: string, focused: boolean): void;
} {
  const order: string[] = [];
  const autoFocusable = new Set<string>();
  let active: string | null = null;
  let enabled = true;
  const subs = new Set<() => void>();

  function notify(): void {
    for (const listener of [...subs]) listener();
  }

  function moveBy(delta: number): void {
    if (!enabled || order.length === 0) return;
    const currentIdx = active === null ? -1 : order.indexOf(active);
    const next = order[(currentIdx + delta + order.length) % order.length] ?? null;
    if (next !== active) {
      active = next;
      notify();
    }
  }

  const ctx: FocusContextValue = {
    register(id, autoFocus): void {
      if (!order.includes(id)) order.push(id);
      if (autoFocus) autoFocusable.add(id);
      if (active === null && (autoFocus || autoFocusable.has(id))) {
        active = id;
        notify();
      }
    },
    unregister(id): void {
      const idx = order.indexOf(id);
      if (idx !== -1) order.splice(idx, 1);
      autoFocusable.delete(id);
      if (active === id) {
        active = order[0] ?? null;
        notify();
      }
    },
    isFocused(id): boolean {
      return enabled && active === id;
    },
    subscribe(listener): () => void {
      subs.add(listener);
      return () => {
        subs.delete(listener);
      };
    },
    focusNext(): void {
      moveBy(1);
    },
    focusPrevious(): void {
      moveBy(-1);
    },
    focus(id): void {
      if (order.includes(id) && active !== id) {
        active = id;
        notify();
      }
    },
    enableFocus(): void {
      if (!enabled) {
        enabled = true;
        notify();
      }
    },
    disableFocus(): void {
      if (enabled) {
        enabled = false;
        notify();
      }
    },
  };

  function setFocus(id: string, focused: boolean): void {
    if (focused) ctx.focus(id);
    else if (active === id) {
      active = null;
      notify();
    }
  }

  return { ctx, setFocus };
}

// Convert a raw input string (UTF-8) into (input, InputKey). Recognizes the
// common ANSI escape prefixes Ink emits. The terminal echoes structured
// `aphrody-jsx-input` frames; this is the fallback path for raw pty stdin.
function parseRawInput(raw: string): { input: string; key: InputKey } {
  const key = emptyKey();
  if (raw === "\x1b[A") { key.upArrow = true; return { input: "", key }; }
  if (raw === "\x1b[B") { key.downArrow = true; return { input: "", key }; }
  if (raw === "\x1b[C") { key.rightArrow = true; return { input: "", key }; }
  if (raw === "\x1b[D") { key.leftArrow = true; return { input: "", key }; }
  if (raw === "\x1b[5~") { key.pageUp = true; return { input: "", key }; }
  if (raw === "\x1b[6~") { key.pageDown = true; return { input: "", key }; }
  if (raw === "\x1b[H" || raw === "\x1b[1~") { key.home = true; return { input: "", key }; }
  if (raw === "\x1b[F" || raw === "\x1b[4~") { key.end = true; return { input: "", key }; }
  if (raw === "\x1b[3~") { key.delete = true; return { input: "", key }; }
  if (raw === "\r" || raw === "\n") { key.return = true; return { input: "", key }; }
  if (raw === "\t") { key.tab = true; return { input: "", key }; }
  if (raw === "\x1b") { key.escape = true; return { input: "", key }; }
  if (raw === "\x7f" || raw === "\b") { key.backspace = true; return { input: "", key }; }
  if (raw.length === 1 && raw.charCodeAt(0) < 0x20) {
    key.ctrl = true;
    return { input: String.fromCharCode(raw.charCodeAt(0) + 96), key };
  }
  return { input: raw, key };
}

// render(element, options) — mounts `element` into a fresh aphrody-jsx root.
export function render(element: ReactNode, options: RenderOptions = {}): Instance {
  const target = resolveTarget(options);

  let wsHandle: ReturnType<typeof wsSink> | null = null;
  const sink: OscSink = options.output
    ? options.output
    : target === "ws"
      ? (wsHandle = wsSink(options.wsUrl ?? DEFAULT_WS_URL)).sink
      : ptySink();

  const root: ReconcilerRoot = createRoot(sink);
  // react-reconciler 0.33 signature:
  //   createContainer(containerInfo, tag, hydrationCallbacks, isStrictMode,
  //                   concurrentUpdatesByDefaultOverride, identifierPrefix,
  //                   onUncaughtError, onCaughtError, onRecoverableError,
  //                   onDefaultTransitionIndicator)
  // Older signatures (0.29) used a single onRecoverableError. We pass both
  // tail-positions; older builds ignore the extras.
  const onErr = (err: Error): void => reportError(err);
  const createContainerFn = (aphrodyReconciler as unknown as {
    createContainer: (...args: unknown[]) => unknown;
  }).createContainer;
  const containerFiber = createContainerFn(
    root,
    ConcurrentRoot,
    null,
    false,
    null,
    "@aphrody/jsx",
    onErr,
    onErr,
    onErr,
    null,
  );

  const initialSize: WindowSize = {
    columns: typeof process !== "undefined" && (process.stdout as { columns?: number })?.columns
      ? ((process.stdout as { columns?: number }).columns ?? 80)
      : 80,
    rows: typeof process !== "undefined" && (process.stdout as { rows?: number })?.rows
      ? ((process.stdout as { rows?: number }).rows ?? 24)
      : 24,
  };

  const { ctx: inputCtx, dispatch: dispatchInput } = makeInputContext();
  const { ctx: windowCtx, set: setWindow } = makeWindowSizeContext(initialSize);
  const { ctx: focusCtx, setFocus } = makeFocusContext();

  // App context: exit() resolves the waiter and tears down the container.
  let resolveExit: (() => void) | null = null;
  let exitError: Error | null = null;
  const exitPromise = new Promise<void>((resolve) => {
    resolveExit = resolve;
  });

  const cleanups: Array<() => void> = [];
  let disposed = false;

  function cleanup(): void {
    if (disposed) return;
    disposed = true;
    for (const fn of cleanups.splice(0)) {
      try { fn(); } catch (err) { reportError(err); }
    }
    try { disposeRoot(root); } catch (err) { reportError(err); }
    if (wsHandle) wsHandle.close();
  }

  const appCtx: AppContextValue = {
    exit(err?: Error): void {
      if (err) exitError = err;
      cleanup();
      if (resolveExit) {
        resolveExit();
        resolveExit = null;
      }
    },
    waitUntilExit(): Promise<void> {
      return exitPromise.then(() => {
        if (exitError) throw exitError;
      });
    },
  };

  // Wire stdin → input dispatch. Two parallel paths:
  //   1. Structured `aphrody-jsx-input` OSC frames (the canonical path).
  //   2. Raw bytes (fallback for plain pty stdin without our extension).
  const oscParser = new OscFrameParser();

  function handleFrames(frames: OscFrame[]): void {
    for (const frame of frames) {
      switch (frame.opcode) {
        case "input": {
          const key = emptyKey();
          for (const [k, v] of Object.entries(frame.fields.key)) {
            if (k in key && typeof v === "boolean") {
              (key as unknown as Record<string, boolean>)[k] = v;
            }
          }
          dispatchInput(frame.fields.input, key);
          break;
        }
        case "window-size":
          setWindow({ columns: frame.fields.columns, rows: frame.fields.rows });
          break;
        case "focus":
          setFocus(frame.fields.id, frame.fields.focused);
          break;
        default:
          // mount/update/unmount: terminal-side only.
          break;
      }
    }
  }

  if (options.input) {
    // Test-mode async iterable.
    (async () => {
      for await (const chunk of options.input!) {
        const text = typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk);
        const frames = oscParser.push(text);
        if (frames.length > 0) handleFrames(frames);
        else {
          const parsed = parseRawInput(text);
          dispatchInput(parsed.input, parsed.key);
        }
      }
    })().catch(reportError);
  } else if (target === "pty" && typeof process !== "undefined" && process.stdin) {
    const stdin = process.stdin as NodeJS.ReadStream & { setRawMode?: (mode: boolean) => void };
    const wasRaw = Boolean((stdin as { isRaw?: boolean }).isRaw);
    if (stdin.isTTY && stdin.setRawMode) {
      try { stdin.setRawMode(true); } catch { /* ignore */ }
    }
    const onData = (chunk: Buffer | string): void => {
      const text = typeof chunk === "string" ? chunk : chunk.toString("utf8");
      const frames = oscParser.push(text);
      if (frames.length > 0) handleFrames(frames);
      else {
        const parsed = parseRawInput(text);
        if (options.exitOnCtrlC !== false && parsed.key.ctrl && parsed.input === "c") {
          appCtx.exit();
          return;
        }
        dispatchInput(parsed.input, parsed.key);
      }
    };
    stdin.on("data", onData);
    cleanups.push(() => {
      stdin.off("data", onData);
      if (stdin.isTTY && stdin.setRawMode && !wasRaw) {
        try { stdin.setRawMode(false); } catch { /* ignore */ }
      }
    });
    const onResize = (): void => {
      const cols = (process.stdout as { columns?: number }).columns ?? initialSize.columns;
      const rows = (process.stdout as { rows?: number }).rows ?? initialSize.rows;
      setWindow({ columns: cols, rows: rows });
    };
    process.stdout.on("resize", onResize);
    cleanups.push(() => process.stdout.off("resize", onResize));
  }

  function withProviders(node: ReactNode): ReactNode {
    return createElement(
      AppContext.Provider,
      { value: appCtx },
      createElement(
        StdoutContext.Provider,
        { value: sink },
        createElement(
          InputContext.Provider,
          { value: inputCtx },
          createElement(
            WindowSizeContext.Provider,
            { value: windowCtx },
            createElement(FocusContext.Provider, { value: focusCtx }, node),
          ),
        ),
      ),
    );
  }

  const updateContainerFn = (aphrodyReconciler as unknown as {
    updateContainer: (
      element: ReactNode,
      container: unknown,
      parent?: unknown,
      callback?: (() => void) | null,
    ) => unknown;
  }).updateContainer;

  updateContainerFn(withProviders(element), containerFiber, null, null);

  return {
    rerender(next: ReactNode): void {
      updateContainerFn(withProviders(next), containerFiber, null, null);
    },
    unmount(): void {
      updateContainerFn(null, containerFiber, null, () => {
        appCtx.exit();
      });
    },
    waitUntilExit: appCtx.waitUntilExit,
    cleanup,
  };
}
