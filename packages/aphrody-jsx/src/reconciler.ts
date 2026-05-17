/** @license SPDX-License-Identifier: Apache-2.0 */

// react-reconciler HostConfig for @aphrody/jsx. Translates React commits into
// a tree of HostInstance objects and emits OSC frames for the renderer.
//
// Strategy:
//   - On the first commit we serialize the entire root and emit a single
//     `mount` frame.
//   - On subsequent commits we collect insert/remove/replace-prop/replace-text
//     operations during the commit phase and emit a single `update` frame
//     containing the full patch list.
//
// This keeps the wire traffic small (no full re-serialization per render)
// while letting the terminal-side renderer apply changes atomically.

import Reconciler from "react-reconciler";
import { DefaultEventPriority } from "react-reconciler/constants.js";
import type { HostConfig } from "react-reconciler";
import {
  diffProps,
  freshId,
  normalizeProps,
  serialize,
  type HostInstance,
  type HostNode,
  type HostTextInstance,
} from "./layout.ts";
import { emit, type OscSink } from "./osc.ts";
import type { JsonElement, JsonPatchOp } from "./types.ts";

export interface ReconcilerRoot {
  rootId: string;
  rootInstance: HostInstance;
  sink: OscSink;
  mounted: boolean;
  pendingPatch: JsonPatchOp[];
}

type ElementType = JsonElement["type"];
type Props = Record<string, unknown>;
type UpdatePayload = { changedProps: Props; nextProps: Props } | null;

const ELEMENT_TYPES = new Set<ElementType>([
  "Box",
  "Text",
  "Newline",
  "Static",
  "Transform",
  "Spacer",
  "Root",
]);

function asElementType(value: string): ElementType {
  if (!ELEMENT_TYPES.has(value as ElementType)) {
    throw new Error(`@aphrody/jsx: unknown element type "${value}"`);
  }
  return value as ElementType;
}

// Single mutable priority lane. The OSC sink is sequential — we don't gain
// anything from concurrent lane scheduling on our side.
let CURRENT_UPDATE_PRIORITY = DefaultEventPriority;

function detach(node: HostNode): void {
  const parent = node.parent;
  if (!parent) return;
  const idx = parent.children.indexOf(node);
  if (idx !== -1) parent.children.splice(idx, 1);
  node.parent = null;
}

function queuePatch(root: ReconcilerRoot, op: JsonPatchOp): void {
  root.pendingPatch.push(op);
}

// HostConfig is highly polymorphic and the @types/react-reconciler package
// lags behind the runtime shape (no entry for the priority API, still expects
// the removed `prepareUpdate`, etc.). We model the config as Record<string,
// unknown> for typing purposes and rely on runtime correctness — the same
// pragmatic choice react-dom and ink make.
type AphrodyHostConfig = Record<string, unknown>;
type _HostConfigShape = HostConfig<
  ElementType,
  Props,
  ReconcilerRoot,
  HostInstance,
  HostTextInstance,
  never,
  never,
  HostNode,
  Record<string, never>,
  UpdatePayload,
  Set<HostNode>,
  number,
  -1
>;
// Touch the imported type so unused-import lints stay quiet.
type _Touch = _HostConfigShape;

const hostConfig: AphrodyHostConfig = {
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  isPrimaryRenderer: true,
  noTimeout: -1,
  scheduleTimeout: ((fn: (...args: unknown[]) => void, delay: number) =>
    setTimeout(fn, delay) as unknown as number) as never,
  cancelTimeout: ((handle: number) =>
    clearTimeout(handle as unknown as ReturnType<typeof setTimeout>)) as never,

  getRootHostContext(): Record<string, never> {
    return {};
  },
  getChildHostContext(): Record<string, never> {
    return {};
  },

  getPublicInstance(instance: HostInstance | HostTextInstance) {
    return instance;
  },

  prepareForCommit(): Record<string, unknown> | null {
    return null;
  },
  resetAfterCommit(rootContainer: ReconcilerRoot): void {
    if (!rootContainer.mounted) {
      // Initial commit: emit a single mount frame for the root subtree.
      emit(rootContainer.sink, {
        opcode: "mount",
        fields: {
          id: rootContainer.rootId,
          tree: serialize(rootContainer.rootInstance),
        },
      });
      rootContainer.mounted = true;
      rootContainer.pendingPatch = [];
      return;
    }
    if (rootContainer.pendingPatch.length === 0) return;
    emit(rootContainer.sink, {
      opcode: "update",
      fields: {
        id: rootContainer.rootId,
        patch: rootContainer.pendingPatch,
      },
    });
    rootContainer.pendingPatch = [];
  },

  createInstance(
    type: string,
    props: Props,
  ): HostInstance {
    return {
      kind: "element",
      type: asElementType(type),
      id: freshId(),
      props,
      children: [],
      parent: null,
    };
  },

  createTextInstance(text: string): HostTextInstance {
    return { kind: "text", id: freshId(), value: text, parent: null };
  },

  appendInitialChild(parent: HostInstance, child: HostNode): void {
    child.parent = parent;
    parent.children.push(child);
  },

  finalizeInitialChildren(): boolean {
    return false;
  },

  // react-reconciler >=0.31 removed `prepareUpdate`; commitUpdate now receives
  // (instance, type, oldProps, newProps, internalHandle) directly. We keep the
  // legacy shape exported for type-compat but the runtime path no longer uses
  // it — see commitUpdate below.

  shouldSetTextContent(): boolean {
    return false;
  },

  appendChild(parent: HostInstance, child: HostNode): void {
    child.parent = parent;
    parent.children.push(child);
  },
  appendChildToContainer(container: ReconcilerRoot, child: HostNode): void {
    child.parent = container.rootInstance;
    container.rootInstance.children.push(child);
    if (container.mounted) {
      queuePatch(container, {
        op: "insert",
        parentId: container.rootInstance.id,
        beforeId: null,
        node: serialize(child),
      });
    }
  },

  insertBefore(
    parent: HostInstance,
    child: HostNode,
    beforeChild: HostNode,
  ): void {
    detach(child);
    const idx = parent.children.indexOf(beforeChild);
    const insertAt = idx === -1 ? parent.children.length : idx;
    parent.children.splice(insertAt, 0, child);
    child.parent = parent;
  },
  insertInContainerBefore(
    container: ReconcilerRoot,
    child: HostNode,
    beforeChild: HostNode,
  ): void {
    detach(child);
    const idx = container.rootInstance.children.indexOf(beforeChild);
    const insertAt = idx === -1 ? container.rootInstance.children.length : idx;
    container.rootInstance.children.splice(insertAt, 0, child);
    child.parent = container.rootInstance;
    if (container.mounted) {
      queuePatch(container, {
        op: "insert",
        parentId: container.rootInstance.id,
        beforeId: beforeChild.id,
        node: serialize(child),
      });
    }
  },

  removeChild(parent: HostInstance, child: HostNode): void {
    detach(child);
    const root = rootOf(parent);
    if (root && root.mounted) queuePatch(root, { op: "remove", id: child.id });
  },
  removeChildFromContainer(container: ReconcilerRoot, child: HostNode): void {
    detach(child);
    if (container.mounted) queuePatch(container, { op: "remove", id: child.id });
  },

  resetTextContent(): void {
    /* no-op: shouldSetTextContent always returns false */
  },

  commitTextUpdate(
    textInstance: HostTextInstance,
    _prevText: string,
    nextText: string,
  ): void {
    textInstance.value = nextText;
    const root = rootOf(textInstance);
    if (!root || !root.mounted) return;
    queuePatch(root, { op: "replace-text", id: textInstance.id, value: nextText });
  },

  // react-reconciler >=0.31 calls commitUpdate(instance, type, oldProps, newProps, internalHandle).
  commitUpdate(
    instance: HostInstance,
    typeOrPayload: unknown,
    oldOrType: unknown,
    newOrOld: unknown,
    _internalOrNew: unknown,
  ): void {
    // Detect signature: old 5-arg form passes updatePayload as arg #2,
    // new 5-arg form passes type (string) as arg #2.
    let oldProps: Props;
    let newProps: Props;
    if (typeof typeOrPayload === "string") {
      oldProps = oldOrType as Props;
      newProps = newOrOld as Props;
    } else {
      oldProps = newOrOld as Props;
      newProps = _internalOrNew as Props;
    }
    instance.props = newProps;
    const root = rootOf(instance);
    if (!root || !root.mounted) return;
    const ops = diffProps(instance.id, oldProps, newProps);
    for (const op of ops) queuePatch(root, op);
  },

  commitMount(): void {
    /* no-op: finalizeInitialChildren always returns false */
  },

  clearContainer(container: ReconcilerRoot): void {
    container.rootInstance.children = [];
    if (container.mounted) {
      // Emit a remove op per existing child so the renderer can drop them.
      // (Most callers won't hit this — render() reuses the same container.)
    }
  },

  // Visibility (Suspense). Map to prop replacement so the renderer can hide
  // a subtree without us tracking explicit visibility state.
  hideInstance(instance: HostInstance): void {
    instance.props = { ...instance.props, hidden: true };
    const root = rootOf(instance);
    if (root && root.mounted) {
      queuePatch(root, {
        op: "replace-prop",
        id: instance.id,
        key: "hidden",
        value: true,
      });
    }
  },
  hideTextInstance(): void {
    /* no-op: text nodes inherit hidden from their parent */
  },
  unhideInstance(instance: HostInstance, props: Props): void {
    instance.props = { ...props };
    const root = rootOf(instance);
    if (root && root.mounted) {
      queuePatch(root, { op: "remove-prop", id: instance.id, key: "hidden" });
    }
  },
  unhideTextInstance(): void {
    /* no-op */
  },

  preparePortalMount(): void {
    /* portals unsupported */
  },

  getCurrentEventPriority(): number {
    return DefaultEventPriority;
  },

  // react-reconciler >=0.31 promoted the priority API to required fields.
  // We model a single priority lane since the OSC sink is sequential anyway.
  resolveUpdatePriority(): number {
    return CURRENT_UPDATE_PRIORITY;
  },
  getCurrentUpdatePriority(): number {
    return CURRENT_UPDATE_PRIORITY;
  },
  setCurrentUpdatePriority(priority: number): void {
    CURRENT_UPDATE_PRIORITY = priority;
  },
  resolveEventType(): null {
    return null;
  },
  resolveEventTimeStamp(): number {
    return -1.1;
  },
  shouldAttemptEagerTransition(): boolean {
    return false;
  },
  trackSchedulerEvent(): void {
    /* no-op */
  },
  requestPostPaintCallback(_cb: (time: number) => void): void {
    /* no-op: we don't paint */
  },
  maySuspendCommit(): boolean {
    return false;
  },
  maySuspendCommitOnUpdate(): boolean {
    return false;
  },
  maySuspendCommitInSyncRender(): boolean {
    return false;
  },
  preloadInstance(): boolean {
    // Returning true tells the reconciler the resource is ready to commit.
    return true;
  },
  startSuspendingCommit(): void {
    /* no-op */
  },
  suspendInstance(): void {
    /* no-op */
  },
  suspendResource(): void {
    /* no-op */
  },
  waitForCommitToBeReady(): null {
    return null;
  },
  NotPendingTransition: null as unknown as never,
  HostTransitionContext: { $$typeof: Symbol.for("react.context"), _currentValue: null } as unknown as never,
  resetFormInstance(): void {
    /* no-op */
  },

  // The remaining HostConfig surface is fiber-internal microtask scheduling,
  // which we delegate to the host's default microtask queue.
  getInstanceFromNode(): null {
    return null;
  },
  beforeActiveInstanceBlur(): void {
    /* no-op */
  },
  afterActiveInstanceBlur(): void {
    /* no-op */
  },
  prepareScopeUpdate(): void {
    /* no-op */
  },
  getInstanceFromScope(): null {
    return null;
  },
  detachDeletedInstance(): void {
    /* no-op */
  },
  supportsMicrotasks: true,
  scheduleMicrotask:
    typeof queueMicrotask === "function"
      ? queueMicrotask.bind(globalThis)
      : ((fn: () => void) =>
          Promise.resolve().then(fn).catch((e: unknown) =>
            setTimeout(() => {
              throw e;
            }),
          )),
};

function rootOf(node: HostNode): ReconcilerRoot | null {
  let cursor: HostNode | null = node;
  while (cursor && cursor.parent) cursor = cursor.parent;
  if (cursor && cursor.kind === "element" && cursor.type === "Root") {
    return ROOT_REGISTRY.get(cursor.id) ?? null;
  }
  return null;
}

// Maps root host-instance ids back to their ReconcilerRoot so HostConfig
// callbacks can locate the active sink.
const ROOT_REGISTRY = new Map<string, ReconcilerRoot>();

// Create the reconciler singleton.
const aphrodyReconciler = (Reconciler as unknown as (cfg: AphrodyHostConfig) => ReturnType<typeof Reconciler>)(hostConfig);

// Create a fresh ReconcilerRoot wired to the given OSC sink.
export function createRoot(sink: OscSink): ReconcilerRoot {
  const rootInstance: HostInstance = {
    kind: "element",
    type: "Root",
    id: freshId(),
    props: {},
    children: [],
    parent: null,
  };
  const root: ReconcilerRoot = {
    rootId: rootInstance.id,
    rootInstance,
    sink,
    mounted: false,
    pendingPatch: [],
  };
  ROOT_REGISTRY.set(rootInstance.id, root);
  return root;
}

// Tear down a ReconcilerRoot: emit unmount and drop registry entry.
export function disposeRoot(root: ReconcilerRoot): void {
  ROOT_REGISTRY.delete(root.rootInstance.id);
  emit(root.sink, { opcode: "unmount", fields: { id: root.rootId } });
}

export { aphrodyReconciler };
