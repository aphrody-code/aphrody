/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, isServer, LitElement, nothing, TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { repeat } from "lit/directives/repeat.js";

import { TreeItem } from "./tree-item.js";

/** Selection behaviour of an `<md-tree>`. */
export type TreeSelectionMode = "single" | "multiple";

/** `detail` of the `tree:select` event. */
export interface TreeSelectEventDetail {
  /** The value that triggered the change (`''` for bulk select/clear). */
  value: string;
  /** The full set of currently selected values. */
  values: string[];
}

/** `detail` of the `tree:label-edit` event. */
export interface TreeLabelEditEventDetail {
  /** The edited item's stable value. */
  itemValue: string;
  /** The previous label (before the edit). */
  value: string;
  /** The newly committed label. */
  newLabel: string;
}

/**
 * A data-driven tree node. Pass an array of these to `<md-tree>.items` instead
 * of slotting `<md-tree-item>` elements. `children` nests recursively.
 */
export interface TreeNode {
  /** Stable value identifying the node (used for selection). */
  value: string;
  /** Display label. */
  label: string;
  /** Optional Material icon name rendered as a leading `md-icon`-style glyph. */
  icon?: string;
  /** Whether the node starts expanded. */
  expanded?: boolean;
  /** Whether the node is disabled. */
  disabled?: boolean;
  /** Child nodes. */
  children?: TreeNode[];
}

/**
 * A tree displays a hierarchical list of `<md-tree-item>` nodes (slotted, or
 * supplied data-driven via the `items` property). It owns keyboard navigation
 * (roving `tabindex`), expand/collapse and selection across the whole subtree,
 * implementing the WAI-ARIA tree pattern (`role="tree"` with `role="treeitem"`
 * descendants and `aria-selected` / `aria-expanded`).
 *
 * Selection supports `single` (default, backward compatible) and `multiple`
 * modes. In multiple mode, `checkboxes` renders an `md-checkbox` per row,
 * selecting a parent cascades to its descendants, and a parent with a partial
 * selection shows the indeterminate state.
 *
 * Keyboard: ArrowUp / ArrowDown move between *visible* items; ArrowRight
 * expands (or descends into) a node; ArrowLeft collapses (or ascends to the
 * parent); Home / End jump to the first / last visible item; `*` expands all
 * siblings of the focused node; Enter / Space activate (toggle selection of)
 * the focused item; in multiple mode Ctrl+A selects all; printable characters
 * type-ahead to the next matching label.
 *
 * Note: drag-and-drop reordering is intentionally out of scope.
 *
 * @fires tree:select {CustomEvent<{value: string, values: string[]}>} Fired
 *     when the selection changes.
 * @fires tree:label-edit {CustomEvent<TreeLabelEditEventDetail>} Fired when an
 *     item's label is committed after an inline edit (only when `editable`).
 */
export class Tree extends LitElement {
  /**
   * The `value` of the currently selected tree item, if any (single-select).
   * In multiple mode this reflects the most recently toggled value.
   */
  @property() selected = "";

  /** Selection behaviour: `single` (default) or `multiple`. */
  @property() selectionMode: TreeSelectionMode = "single";

  /** Whether to render a leading `md-checkbox` per row (multi-select affordance). */
  @property({ type: Boolean }) checkboxes = false;

  /**
   * When true, item labels may be edited inline (F2 or double-click enters edit
   * mode; Enter/blur commits, Escape cancels). Works in both slotted and
   * data-driven (`items`) modes; commits emit `tree:label-edit`.
   */
  @property({ type: Boolean }) editable = false;

  /**
   * Optional data-driven node tree. When set (non-empty), the tree renders
   * `<md-tree-item>` elements from this array instead of slotted children.
   */
  @property({ attribute: false }) items: TreeNode[] = [];

  /** The set of currently selected values (authoritative in multiple mode). */
  @state() private selectedSet = new Set<string>();

  /** The value of the item currently being edited inline (`null` = none). */
  @state() private editingValue: string | null = null;

  override connectedCallback() {
    super.connectedCallback();
    if (!isServer) {
      this.addEventListener("tree-item:activate", this.handleActivate as EventListener);
      this.addEventListener("tree-item:edit-request", this.handleEditRequest as EventListener);
      this.addEventListener("tree-item:edit-commit", this.handleEditCommit as EventListener);
      this.addEventListener("tree-item:edit-cancel", this.handleEditCancel as EventListener);
      this.addEventListener("keydown", this.handleKeydown);
      void this.updateComplete.then(() => {
        this.refreshStructure();
      });
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    if (!isServer) {
      this.removeEventListener("tree-item:activate", this.handleActivate as EventListener);
      this.removeEventListener("tree-item:edit-request", this.handleEditRequest as EventListener);
      this.removeEventListener("tree-item:edit-commit", this.handleEditCommit as EventListener);
      this.removeEventListener("tree-item:edit-cancel", this.handleEditCancel as EventListener);
      this.removeEventListener("keydown", this.handleKeydown);
      // Cancel any pending type-ahead reset so a detached tree leaves no timer.
      if (this.typeaheadTimer) {
        clearTimeout(this.typeaheadTimer);
        this.typeaheadTimer = 0;
      }
    }
  }

  override willUpdate(changed: Map<string, unknown>) {
    if (changed.has("selected") && this.selectionMode === "single") {
      this.selectedSet = this.selected ? new Set([this.selected]) : new Set();
    }
  }

  override updated(changed: Map<string, unknown>) {
    if (
      changed.has("items") ||
      changed.has("checkboxes") ||
      changed.has("editable") ||
      changed.has("editingValue")
    ) {
      void this.updateComplete.then(() => {
        this.refreshStructure();
      });
    }
  }

  /** Returns every `<md-tree-item>` in the subtree, in document order. */
  get treeItems(): TreeItem[] {
    return this.dataDriven
      ? Array.from(this.renderRoot.querySelectorAll("md-tree-item"))
      : Array.from(this.querySelectorAll("md-tree-item"));
  }

  /** The list of currently selected values. */
  get selectedValues(): string[] {
    return Array.from(this.selectedSet);
  }

  set selectedValues(values: string[]) {
    this.selectedSet = new Set(values);
    this.applySelectionToItems();
  }

  private get dataDriven(): boolean {
    return this.items != null && this.items.length > 0;
  }

  protected override render() {
    return html`
      <div
        role="tree"
        class="tree"
        aria-multiselectable=${this.selectionMode === "multiple" ? "true" : nothing}
      >
        ${this.dataDriven
          ? this.items.map((node) => this.renderNode(node))
          : html`<slot @slotchange=${this.handleSlotChange}></slot>`}
      </div>
    `;
  }

  private renderNode(node: TreeNode): TemplateResult {
    return html`
      <md-tree-item
        .label=${node.label}
        .value=${node.value}
        ?expanded=${node.expanded ?? false}
        ?disabled=${node.disabled ?? false}
        ?editable=${this.editable}
        ?editing=${this.editingValue === node.value}
      >
        ${node.icon ? html`<span slot="icon" class="data-icon">${node.icon}</span>` : nothing}
        ${node.children
          ? repeat(
              node.children,
              (child) => child.value,
              (child) => this.renderNode(child),
            )
          : nothing}
      </md-tree-item>
    `;
  }

  private handleSlotChange() {
    this.refreshStructure();
  }

  /**
   * Recomputes each item's nesting `level`, propagates checkbox mode, applies
   * the current selection, computes parent indeterminate states, and installs a
   * single roving `tabindex` (the first selected item, or the first item).
   */
  private refreshStructure() {
    const roots = this.rootItems;
    for (const root of roots) {
      this.assignLevels(root, 0);
    }
    const items = this.treeItems;
    for (const item of items) {
      item.hasCheckbox = this.checkboxes && this.selectionMode === "multiple";
      item.editable = this.editable;
      item.editing = this.editingValue === item.value;
      item.tabIndex = -1;
    }
    this.applySelectionToItems();
    const focusTarget = items.find((item) => this.selectedSet.has(item.value)) ?? items[0];
    if (focusTarget) {
      focusTarget.tabIndex = 0;
    }
  }

  private get rootItems(): TreeItem[] {
    const scope = this.dataDriven
      ? (this.renderRoot.querySelector(".tree") as ParentNode | null)
      : this;
    if (!scope) {
      return [];
    }
    return Array.from(scope.children).filter((el): el is TreeItem => el instanceof TreeItem);
  }

  private assignLevels(item: TreeItem, level: number) {
    item.level = level;
    for (const child of item.childItems) {
      this.assignLevels(child, level + 1);
    }
  }

  /** Pushes the selected/indeterminate flags onto every item. */
  private applySelectionToItems() {
    for (const item of this.treeItems) {
      item.selected = this.selectedSet.has(item.value);
      item.indeterminate = false;
    }
    if (this.selectionMode === "multiple") {
      for (const root of this.rootItems) {
        this.computeIndeterminate(root);
      }
    }
  }

  /**
   * Computes, bottom-up, whether each parent is fully selected (selected,
   * not indeterminate), partially selected (indeterminate), or unselected.
   * Returns `'all' | 'some' | 'none'` for the subtree rooted at `item`.
   */
  private computeIndeterminate(item: TreeItem): "all" | "some" | "none" {
    if (!item.hasChildren) {
      return this.selectedSet.has(item.value) ? "all" : "none";
    }
    let allCount = 0;
    let someCount = 0;
    const children = item.childItems;
    for (const child of children) {
      const state = this.computeIndeterminate(child);
      if (state === "all") {
        allCount++;
      } else if (state === "some") {
        someCount++;
      }
    }
    if (allCount === children.length && someCount === 0) {
      item.selected = true;
      item.indeterminate = false;
      this.selectedSet.add(item.value);
      return "all";
    }
    if (allCount === 0 && someCount === 0) {
      item.selected = false;
      item.indeterminate = false;
      this.selectedSet.delete(item.value);
      return "none";
    }
    item.selected = false;
    item.indeterminate = true;
    this.selectedSet.delete(item.value);
    return "some";
  }

  /** Visible items = those whose every ancestor tree item is expanded. */
  private get visibleItems(): TreeItem[] {
    return this.treeItems.filter((item) => this.isVisible(item));
  }

  private isVisible(item: TreeItem): boolean {
    let parent = item.parentElement;
    while (parent && parent !== this) {
      if (parent instanceof TreeItem && !parent.expanded) {
        return false;
      }
      parent = parent.parentElement;
    }
    return true;
  }

  private readonly handleActivate = (event: CustomEvent<{ value: string }>) => {
    const value = event.detail.value;
    const target = this.treeItems.find((item) => item.value === value);
    if (!target || target.disabled) {
      return;
    }
    if (this.selectionMode === "multiple") {
      this.toggleValue(value);
    } else {
      this.selectValue(value);
    }
    this.focusItem(target);
  };

  // ----------------------------------------------------------- Label editing

  private readonly handleEditRequest = (event: CustomEvent<{ value: string }>) => {
    if (!this.editable) {
      return;
    }
    const target = this.treeItems.find((i) => i.value === event.detail.value);
    if (!target || target.disabled) {
      return;
    }
    this.editingValue = event.detail.value;
  };

  private readonly handleEditCommit = (event: CustomEvent<{ value: string; newLabel: string }>) => {
    const { value, newLabel } = event.detail;
    if (this.editingValue !== value) {
      return;
    }
    this.editingValue = null;
    const item = this.treeItems.find((i) => i.value === value);
    const oldLabel = item ? item.label : "";
    if (newLabel === oldLabel) {
      return;
    }
    if (this.dataDriven) {
      // Update the backing node so the new label survives re-renders.
      const node = this.findNode(this.items, value);
      if (node) {
        node.label = newLabel;
      }
      this.requestUpdate();
    } else if (item) {
      item.label = newLabel;
    }
    this.dispatchEvent(
      new CustomEvent<TreeLabelEditEventDetail>("tree:label-edit", {
        detail: { itemValue: value, value: oldLabel, newLabel },
        bubbles: true,
        composed: true,
      }),
    );
  };

  private readonly handleEditCancel = (event: CustomEvent<{ value: string }>) => {
    if (this.editingValue === event.detail.value) {
      this.editingValue = null;
    }
  };

  /** Depth-first search for a node by value in a data-driven tree. */
  private findNode(nodes: TreeNode[], value: string): TreeNode | null {
    for (const node of nodes) {
      if (node.value === value) {
        return node;
      }
      if (node.children) {
        const found = this.findNode(node.children, value);
        if (found) {
          return found;
        }
      }
    }
    return null;
  }

  /** Single-select: replace the selection with `value`. */
  private selectValue(value: string) {
    this.selected = value;
    this.selectedSet = new Set([value]);
    this.applySelectionToItems();
    this.emitSelect(value);
  }

  /** Multi-select: toggle `value` and cascade to descendants. */
  private toggleValue(value: string) {
    const item = this.treeItems.find((i) => i.value === value);
    if (!item) {
      return;
    }
    // A partially-selected parent becomes fully selected on toggle; otherwise
    // flip its current selected state.
    const nextSelected = !(item.selected && !item.indeterminate);
    this.setSubtreeSelected(item, nextSelected);
    this.selected = value;
    this.applySelectionToItems();
    this.emitSelect(value);
  }

  private setSubtreeSelected(item: TreeItem, selected: boolean) {
    if (!item.disabled) {
      if (selected) {
        this.selectedSet.add(item.value);
      } else {
        this.selectedSet.delete(item.value);
      }
    }
    for (const child of item.childItems) {
      this.setSubtreeSelected(child, selected);
    }
  }

  private emitSelect(value: string): void {
    this.dispatchEvent(
      new CustomEvent<TreeSelectEventDetail>("tree:select", {
        detail: { value, values: this.selectedValues },
        bubbles: true,
        composed: true,
      }),
    );
  }

  /** Expands every node with children. */
  expandAll() {
    for (const item of this.treeItems) {
      if (item.hasChildren) {
        item.expanded = true;
      }
    }
  }

  /** Collapses every node. */
  collapseAll() {
    for (const item of this.treeItems) {
      item.expanded = false;
    }
  }

  /** Selects every (non-disabled) node. Multi-select only. */
  selectAll() {
    if (this.selectionMode !== "multiple") {
      return;
    }
    this.selectedSet = new Set(this.treeItems.filter((i) => !i.disabled).map((i) => i.value));
    this.applySelectionToItems();
    this.emitSelect("");
  }

  /** Clears the selection. */
  clearSelection() {
    this.selected = "";
    this.selectedSet = new Set();
    this.applySelectionToItems();
    this.emitSelect("");
  }

  private readonly handleKeydown = (event: KeyboardEvent) => {
    if (
      this.selectionMode === "multiple" &&
      (event.ctrlKey || event.metaKey) &&
      event.key.toLowerCase() === "a"
    ) {
      event.preventDefault();
      this.selectAll();
      return;
    }
    const current = this.activeItem(event);
    if (!current) {
      return;
    }
    const visible = this.visibleItems;
    const index = visible.indexOf(current);
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        this.focusByIndex(visible, index + 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        this.focusByIndex(visible, index - 1);
        break;
      case "ArrowRight":
        event.preventDefault();
        this.handleArrowRight(current, visible, index);
        break;
      case "ArrowLeft":
        event.preventDefault();
        this.handleArrowLeft(current);
        break;
      case "Home":
        event.preventDefault();
        this.focusByIndex(visible, 0);
        break;
      case "End":
        event.preventDefault();
        this.focusByIndex(visible, visible.length - 1);
        break;
      case "*":
        event.preventDefault();
        this.expandSiblings(current);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (current.disabled) {
          break;
        }
        if (this.selectionMode === "multiple") {
          this.toggleValue(current.value);
        } else {
          this.selectValue(current.value);
        }
        break;
      default:
        this.handleTypeahead(event, visible, index);
        break;
    }
  };

  /** Expands all siblings of `item` at the same level. */
  private expandSiblings(item: TreeItem) {
    const parent = item.parentElement;
    const scope = parent instanceof TreeItem ? parent.childItems : this.rootItems;
    for (const sibling of scope) {
      if (sibling.hasChildren) {
        sibling.expanded = true;
      }
    }
  }

  private lastTypeahead = "";
  private typeaheadTimer = 0;

  private handleTypeahead(event: KeyboardEvent, visible: TreeItem[], index: number) {
    if (event.key.length !== 1 || event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    const char = event.key.toLowerCase();
    if (!/\S/.test(char)) {
      return;
    }
    event.preventDefault();
    if (!isServer) {
      clearTimeout(this.typeaheadTimer);
      this.typeaheadTimer = window.setTimeout(() => {
        this.lastTypeahead = "";
      }, 500);
    }
    this.lastTypeahead += char;
    const query = this.lastTypeahead;
    // Search after the current index, wrapping around.
    for (let offset = 1; offset <= visible.length; offset++) {
      const candidate = visible[(index + offset) % visible.length];
      if (candidate.label.toLowerCase().startsWith(query)) {
        this.focusItem(candidate);
        return;
      }
    }
  }

  /** Resolves the focused tree item from the event's composed path. */
  private activeItem(event: Event): TreeItem | undefined {
    const path = event.composedPath();
    for (const node of path) {
      if (node instanceof TreeItem) {
        return node;
      }
    }
    return this.treeItems.find((item) => item.tabIndex === 0);
  }

  private handleArrowRight(current: TreeItem, visible: TreeItem[], index: number) {
    if (current.hasChildren && !current.expanded) {
      current.toggle();
    } else if (current.hasChildren && current.expanded) {
      this.focusByIndex(visible, index + 1);
    }
  }

  private handleArrowLeft(current: TreeItem) {
    if (current.hasChildren && current.expanded) {
      current.toggle();
      return;
    }
    let parent = current.parentElement;
    while (parent && parent !== this) {
      if (parent instanceof TreeItem) {
        this.focusItem(parent);
        return;
      }
      parent = parent.parentElement;
    }
  }

  private focusByIndex(items: TreeItem[], index: number) {
    if (index < 0 || index >= items.length) {
      return;
    }
    this.focusItem(items[index]);
  }

  private focusItem(item: TreeItem) {
    for (const other of this.treeItems) {
      other.tabIndex = -1;
    }
    item.tabIndex = 0;
    item.focus();
  }
}
