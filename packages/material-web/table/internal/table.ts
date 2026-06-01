/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { html, LitElement, nothing, type TemplateResult } from "lit";
import { property, state } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import { styleMap } from "lit/directives/style-map.js";
import { repeat } from "lit/directives/repeat.js";

/** Sort direction for a data table column. `''` means unsorted. */
export type TableSortDirection = "asc" | "desc" | "";

/**
 * Supported per-column filter input types. `text` is a free-text `contains`
 * match; `number` exposes equals/greater-than/less-than operators; `''`
 * disables column filtering for that column.
 */
export type TableFilterType = "text" | "number" | "";

/** Comparison operators usable by a column filter. */
export type TableFilterOperator =
  | "contains"
  | "equals"
  | "startsWith"
  | "endsWith"
  | "gt"
  | "lt"
  | "gte"
  | "lte";

/** A single column descriptor for {@link Table}. */
export interface TableColumn {
  /** Key into each row's record used to read the cell value. */
  key: string;
  /** Human-readable header label. */
  label: string;
  /** Whether clicking the header toggles sorting on this column. */
  sortable?: boolean;
  /** Whether the column holds numeric data (right-aligned, numeric compare). */
  numeric?: boolean;
  /**
   * Filter input type for this column. When set (and the table is `filterable`)
   * a filter control is rendered in the column's filter row. `number` columns
   * gain an operator selector; `text` columns use a `contains` match.
   */
  filter?: TableFilterType;
  /** Whether the column may be resized by dragging its trailing edge. */
  resizable?: boolean;
  /** Explicit column width (CSS length, e.g. `'160px'`). Overridden by resize. */
  width?: string;
  /**
   * When true, a cell in this column may be edited inline (double-click, or
   * Enter on the focused cell). Committing emits {@link Table}'s
   * `table:cell-edit` event and updates the displayed value in place.
   */
  editable?: boolean;
}

/** A single data row: an arbitrary record keyed by column `key`. */
export type TableRow = Record<string, unknown>;

/** An active per-column filter. */
export interface TableColumnFilter {
  /** Column key the filter applies to. */
  key: string;
  /** Comparison operator. */
  operator: TableFilterOperator;
  /** Filter value (string for text, number-as-string for numeric). */
  value: string;
}

/** A single sort criterion (used for multi-column sort). */
export interface TableSortCriterion {
  key: string;
  direction: "asc" | "desc";
}

/**
 * `detail` of the `table:sort` event. The legacy single-column fields
 * (`sortKey`/`sortDirection`) reflect the primary (first) sort criterion and
 * remain for backward compatibility; `sort` carries the full multi-column
 * order.
 */
export interface TableSortEventDetail {
  sortKey: string;
  sortDirection: TableSortDirection;
  /** Full ordered list of active sort criteria. */
  sort: TableSortCriterion[];
}

/** `detail` of the `table:filter` event. */
export interface TableFilterEventDetail {
  /** Global quick-filter text (matches across all columns). */
  quickFilter: string;
  /** Active per-column filters. */
  filters: TableColumnFilter[];
}

/** `detail` of the `table:cell-edit` event. */
export interface TableCellEditEventDetail {
  /** Stable identifier of the edited row (see {@link Table.getRowId}). */
  rowId: string | number;
  /** Column key (field) that was edited. */
  field: string;
  /** The newly committed cell value (string from the inline input). */
  value: string;
  /** The full edited row record (already updated in place). */
  row: TableRow;
}

/** `detail` of the `table:page` event. */
export interface TablePageEventDetail {
  /** The (zero-based) page index. */
  pageIndex: number;
  /** The current page size. */
  pageSize: number;
  /** Total number of rows after filtering (the paginated length). */
  length: number;
}

/**
 * `detail` of the `table:selection-change` event.
 */
export interface TableSelectionEventDetail {
  /**
   * Indices (into the current display order, i.e. filtered + sorted + paged)
   * of the selected rows. Kept for backward compatibility.
   */
  selected: number[];
  /** Stable identifiers of the selected rows (see {@link Table.getRowId}). */
  selectedIds: Array<string | number>;
  /** The selected row records. */
  selectedRows: TableRow[];
}

/**
 * A Material 3 data table with Data-Grid-class features. Data-driven: supply
 * {@link columns} and {@link rows} and the element renders a real `<table>`
 * with sortable headers, per-column + global filtering, integrated pagination,
 * row selection, CSV export, and optional column resize/reorder.
 *
 * The display pipeline is: **filter → sort → paginate**. All public read
 * accessors and {@link exportCsv} respect the current filter and sort.
 *
 * Backward compatible: the original `columns`/`rows`/`sortKey`/`sortDirection`/
 * `selectable` props and the `table:sort` / `table:selection-change` events are
 * preserved. New behaviour is opt-in via flags (`filterable`, `paginated`,
 * `reorderable`).
 *
 * @fires table:sort {CustomEvent<TableSortEventDetail>} Fired when the active
 *     sort column(s) or direction changes.
 * @fires table:filter {CustomEvent<TableFilterEventDetail>} Fired when the
 *     quick filter or any column filter changes.
 * @fires table:page {CustomEvent<TablePageEventDetail>} Fired when the page
 *     index or page size changes.
 * @fires table:selection-change {CustomEvent<TableSelectionEventDetail>} Fired
 *     when the set of selected rows changes (only when `selectable`).
 * @fires table:cell-edit {CustomEvent<TableCellEditEventDetail>} Fired when an
 *     editable cell's value is committed (only for `editable` columns).
 */
export class Table extends LitElement {
  /** Column descriptors, in display order. */
  @property({ attribute: false }) columns: TableColumn[] = [];

  /** Row records. Each is read by column `key`. */
  @property({ attribute: false }) rows: TableRow[] = [];

  /** Key of the column currently driving the (primary) sort (`''` unsorted). */
  @property({ attribute: "sort-key" }) sortKey = "";

  /** Direction of the current (primary) sort. */
  @property({ attribute: "sort-direction" }) sortDirection: TableSortDirection = "";

  /**
   * When true, holding Shift while clicking a sortable header appends/updates
   * that column as an additional sort criterion (multi-column sort). When
   * false, sorting is single-column (legacy behaviour) but still stable.
   */
  @property({ type: Boolean, attribute: "multi-sort" }) multiSort = false;

  /** When true, renders a leading checkbox column for row selection. */
  @property({ type: Boolean }) selectable = false;

  /** When true, renders a filter row and a global quick-filter field. */
  @property({ type: Boolean }) filterable = false;

  /** When true, paginates the rows and renders an integrated paginator. */
  @property({ type: Boolean }) paginated = false;

  /** Page size when {@link paginated}. */
  @property({ type: Number, attribute: "page-size" }) pageSize = 10;

  /** Current (zero-based) page index when {@link paginated}. */
  @property({ type: Number, attribute: "page-index" }) pageIndex = 0;

  /** Selectable page sizes shown in the integrated paginator selector. */
  @property({ attribute: false }) rowsPerPageOptions: number[] = [5, 10, 25, 50];

  /** When true, columns may be reordered by dragging their headers. */
  @property({ type: Boolean, attribute: "reorderable" }) reorderable = false;

  /**
   * Optional accessor returning a stable identifier for a row. When provided,
   * selection survives sort/filter/paginate changes. Defaults to the row's
   * `id` field if present, otherwise the row's index in {@link rows}.
   */
  @property({ attribute: false }) getRowId?: (row: TableRow, index: number) => string | number;

  /** Full multi-column sort order. The primary criterion mirrors sortKey. */
  @state() private sortCriteria: TableSortCriterion[] = [];

  /** Global quick-filter text. */
  @state() private quickFilter = "";

  /** Active per-column filters, keyed by column key. */
  @state() private columnFilters = new Map<
    string,
    { operator: TableFilterOperator; value: string }
  >();

  /** Selected row identifiers (stable across sort/filter/paginate). */
  @state() private selectedIds = new Set<string | number>();

  /** Display-order override of column keys for reordering. `null` = source. */
  @state() private columnOrder: string[] | null = null;

  /** Per-column resized widths (CSS length), keyed by column key. */
  @state() private columnWidths = new Map<string, string>();

  private resizeState: {
    key: string;
    startX: number;
    startWidth: number;
  } | null = null;

  private dragKey: string | null = null;

  /**
   * The cell currently being edited, identified by its stable row id and column
   * key. `null` when no cell is in edit mode.
   */
  @state() private editing: { rowId: string | number; key: string } | null = null;

  /** When true, a freshly opened editor should grab focus on next render. */
  private editorNeedsFocus = false;

  // Memoised derived views of the data. Recomputed in `willUpdate` (before
  // render) rather than via getters so the filter → sort → paginate pipeline
  // runs at most once per update — critical on large grids where `render()`
  // reads these several times.
  private memoOrderedColumns: TableColumn[] = [];
  private memoFilteredRows: TableRow[] = [];
  private memoSortedRows: TableRow[] = [];
  private memoPagedRows: TableRow[] = [];

  override willUpdate(changed: Map<string, unknown>) {
    // Keep legacy single-column props in sync with the multi-sort criteria.
    if ((changed.has("sortKey") || changed.has("sortDirection")) && !this.syncingSort) {
      if (this.sortKey && this.sortDirection !== "") {
        const primary: TableSortCriterion = {
          key: this.sortKey,
          direction: this.sortDirection,
        };
        // Preserve any trailing criteria that are not the primary key.
        const rest = this.sortCriteria.filter((c) => c.key !== this.sortKey);
        this.sortCriteria = this.multiSort ? [primary, ...rest] : [primary];
      } else {
        this.sortCriteria = [];
      }
    }

    // Clamp the page before deriving the visible rows so the paginator and the
    // body stay consistent (and `render()` itself can remain pure).
    if (this.paginated) {
      this.clampPage();
    }

    // Recompute the derived pipeline once for this update.
    this.memoOrderedColumns = this.computeOrderedColumns();
    this.memoFilteredRows = this.computeFilteredRows();
    this.memoSortedRows = this.computeSortedRows(this.memoFilteredRows);
    this.memoPagedRows = this.computePagedRows(this.memoSortedRows);
  }

  private syncingSort = false;

  override updated() {
    // Focus (and select) a freshly opened inline editor so typing replaces the
    // current value immediately.
    if (this.editorNeedsFocus && this.editing) {
      this.editorNeedsFocus = false;
      const input = this.renderRoot.querySelector<HTMLInputElement>(".cell-editor");
      if (input) {
        input.focus();
        input.select();
      }
    }
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    // Drop any in-flight column-resize listeners attached to `window` so a
    // detached table never keeps reacting to pointer moves.
    this.onResizeEnd();
  }

  /** Resolve the stable id for a source row at a given source index. */
  private rowId(row: TableRow, sourceIndex: number): string | number {
    if (this.getRowId) {
      return this.getRowId(row, sourceIndex);
    }
    const id = (row as { id?: unknown }).id;
    if (typeof id === "string" || typeof id === "number") {
      return id;
    }
    return sourceIndex;
  }

  /** The effective columns in display order (memoised; see `willUpdate`). */
  private get orderedColumns(): TableColumn[] {
    return this.memoOrderedColumns;
  }

  private computeOrderedColumns(): TableColumn[] {
    if (!this.columnOrder) {
      return this.columns;
    }
    const byKey = new Map(this.columns.map((c) => [c.key, c]));
    const ordered: TableColumn[] = [];
    for (const key of this.columnOrder) {
      const col = byKey.get(key);
      if (col) {
        ordered.push(col);
        byKey.delete(key);
      }
    }
    // Append any columns added after the order was captured.
    for (const col of this.columns) {
      if (byKey.has(col.key)) {
        ordered.push(col);
      }
    }
    return ordered;
  }

  /** Rows after filter+sort+paginate (memoised; see `willUpdate`). */
  private get filteredRows(): TableRow[] {
    return this.memoFilteredRows;
  }

  private computeFilteredRows(): TableRow[] {
    const quick = this.quickFilter.trim().toLowerCase();
    const hasColumnFilters = this.columnFilters.size > 0;
    if (!quick && !hasColumnFilters) {
      return this.rows;
    }
    return this.rows.filter((row) => {
      if (quick) {
        const match = this.columns.some((col) => {
          const v = row[col.key];
          if (v === null || v === undefined) {
            return false;
          }
          return String(v).toLowerCase().includes(quick);
        });
        if (!match) {
          return false;
        }
      }
      for (const [key, { operator, value }] of this.columnFilters) {
        if (value === "") {
          continue;
        }
        const column = this.columns.find((c) => c.key === key);
        if (!this.matchesFilter(row[key], operator, value, column?.numeric)) {
          return false;
        }
      }
      return true;
    });
  }

  private matchesFilter(
    cell: unknown,
    operator: TableFilterOperator,
    value: string,
    numeric?: boolean,
  ): boolean {
    if (numeric || ["gt", "lt", "gte", "lte"].includes(operator)) {
      const cn = typeof cell === "number" ? cell : Number(cell);
      const vn = Number(value);
      if (Number.isNaN(cn) || Number.isNaN(vn)) {
        // Fall back to string compare for non-numeric content.
      } else {
        switch (operator) {
          case "equals":
            return cn === vn;
          case "gt":
            return cn > vn;
          case "lt":
            return cn < vn;
          case "gte":
            return cn >= vn;
          case "lte":
            return cn <= vn;
          default:
            break;
        }
      }
    }
    const cs = cell === null || cell === undefined ? "" : String(cell);
    const csl = cs.toLowerCase();
    const vsl = value.toLowerCase();
    switch (operator) {
      case "equals":
        return csl === vsl;
      case "startsWith":
        return csl.startsWith(vsl);
      case "endsWith":
        return csl.endsWith(vsl);
      case "contains":
      default:
        return csl.includes(vsl);
    }
  }

  private computeSortedRows(rows: TableRow[]): TableRow[] {
    if (this.sortCriteria.length === 0) {
      return rows;
    }
    const columnByKey = new Map(this.columns.map((c) => [c.key, c]));
    // Stable sort via decorate-sort-undecorate to keep original order on ties.
    return rows
      .map((row, i) => ({ row, i }))
      .sort((a, b) => {
        for (const crit of this.sortCriteria) {
          const column = columnByKey.get(crit.key);
          const numeric = column?.numeric ?? false;
          const dir = crit.direction === "desc" ? -1 : 1;
          const c = this.compare(a.row[crit.key], b.row[crit.key], numeric);
          if (c !== 0) {
            return c * dir;
          }
        }
        return a.i - b.i;
      })
      .map((entry) => entry.row);
  }

  /** The rows visible on the current page (memoised; see `willUpdate`). */
  private get pagedRows(): TableRow[] {
    return this.memoPagedRows;
  }

  private computePagedRows(sorted: TableRow[]): TableRow[] {
    if (!this.paginated || this.pageSize <= 0) {
      return sorted;
    }
    const start = this.pageIndex * this.pageSize;
    return sorted.slice(start, start + this.pageSize);
  }

  /** Total pages for the current filtered length and page size. */
  get pageCount(): number {
    if (this.pageSize <= 0) {
      return 0;
    }
    return Math.ceil(this.memoFilteredRows.length / this.pageSize);
  }

  private compare(a: unknown, b: unknown, numeric: boolean): number {
    const aNull = a === null || a === undefined || a === "";
    const bNull = b === null || b === undefined || b === "";
    if (aNull && bNull) {
      return 0;
    }
    if (aNull) {
      return 1;
    }
    if (bNull) {
      return -1;
    }
    if (numeric) {
      const an = typeof a === "number" ? a : Number(a);
      const bn = typeof b === "number" ? b : Number(b);
      if (!Number.isNaN(an) && !Number.isNaN(bn)) {
        return an < bn ? -1 : an > bn ? 1 : 0;
      }
    }
    return String(a).localeCompare(String(b), undefined, { numeric: true });
  }

  // ---------------------------------------------------------------- Sorting

  private handleHeaderClick(column: TableColumn, event?: MouseEvent) {
    if (!column.sortable) {
      return;
    }
    const additive = this.multiSort && !!event?.shiftKey;
    const existing = this.sortCriteria.find((c) => c.key === column.key);
    let next: TableSortCriterion[];
    if (!additive) {
      // Single-column cycle: asc → desc → unsorted.
      if (!existing) {
        next = [{ key: column.key, direction: "asc" }];
      } else if (existing.direction === "asc") {
        next = [{ key: column.key, direction: "desc" }];
      } else {
        next = [];
      }
    } else {
      next = [...this.sortCriteria];
      if (!existing) {
        next.push({ key: column.key, direction: "asc" });
      } else if (existing.direction === "asc") {
        existing.direction = "desc";
      } else {
        next = next.filter((c) => c.key !== column.key);
      }
    }
    this.applySort(next);
  }

  private applySort(next: TableSortCriterion[]) {
    this.sortCriteria = next;
    const primary = next[0];
    this.syncingSort = true;
    this.sortKey = primary ? primary.key : "";
    this.sortDirection = primary ? primary.direction : "";
    this.syncingSort = false;
    // Page indices may now point past the end; clamp.
    this.clampPage();
    this.dispatchEvent(
      new CustomEvent<TableSortEventDetail>("table:sort", {
        detail: {
          sortKey: this.sortKey,
          sortDirection: this.sortDirection,
          sort: next.map((c) => ({ ...c })),
        },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleHeaderKeydown(event: KeyboardEvent, column: TableColumn) {
    if (!column.sortable) {
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      this.handleHeaderClick(column, event as unknown as MouseEvent);
    }
  }

  private ariaSortFor(column: TableColumn): "ascending" | "descending" | "none" {
    const crit = this.sortCriteria.find((c) => c.key === column.key);
    if (!column.sortable || !crit) {
      return "none";
    }
    return crit.direction === "desc" ? "descending" : "ascending";
  }

  // -------------------------------------------------------------- Filtering

  private handleQuickFilter(event: Event) {
    this.quickFilter = (event.target as HTMLInputElement).value;
    this.pageIndex = 0;
    this.emitFilter();
  }

  private handleColumnFilterValue(column: TableColumn, event: Event) {
    const value = (event.target as HTMLInputElement).value;
    const current = this.columnFilters.get(column.key);
    const operator: TableFilterOperator =
      current?.operator ?? (column.numeric ? "equals" : "contains");
    const next = new Map(this.columnFilters);
    if (value === "") {
      next.delete(column.key);
    } else {
      next.set(column.key, { operator, value });
    }
    this.columnFilters = next;
    this.pageIndex = 0;
    this.emitFilter();
  }

  private handleColumnFilterOperator(column: TableColumn, event: Event) {
    const operator = (event.target as HTMLSelectElement).value as TableFilterOperator;
    const current = this.columnFilters.get(column.key);
    const value = current?.value ?? "";
    const next = new Map(this.columnFilters);
    next.set(column.key, { operator, value });
    this.columnFilters = next;
    this.pageIndex = 0;
    this.emitFilter();
  }

  private emitFilter() {
    const filters: TableColumnFilter[] = [];
    for (const [key, { operator, value }] of this.columnFilters) {
      filters.push({ key, operator, value });
    }
    this.dispatchEvent(
      new CustomEvent<TableFilterEventDetail>("table:filter", {
        detail: { quickFilter: this.quickFilter, filters },
        bubbles: true,
        composed: true,
      }),
    );
  }

  // ------------------------------------------------------------- Pagination

  private clampPage() {
    const max = Math.max(0, this.pageCount - 1);
    if (this.pageIndex > max) {
      this.pageIndex = max;
    }
    if (this.pageIndex < 0) {
      this.pageIndex = 0;
    }
  }

  private setPageIndex(index: number) {
    const max = Math.max(0, this.pageCount - 1);
    const next = Math.min(Math.max(0, index), max);
    if (next === this.pageIndex) {
      return;
    }
    this.pageIndex = next;
    this.emitPage();
  }

  private handlePageSizeChange(event: Event) {
    const value = Number((event.target as HTMLSelectElement).value);
    if (!Number.isFinite(value) || value <= 0 || value === this.pageSize) {
      return;
    }
    const firstItem = this.pageIndex * this.pageSize;
    this.pageSize = value;
    this.pageIndex = Math.floor(firstItem / value);
    this.clampPage();
    this.emitPage();
  }

  private emitPage() {
    this.dispatchEvent(
      new CustomEvent<TablePageEventDetail>("table:page", {
        detail: {
          pageIndex: this.pageIndex,
          pageSize: this.pageSize,
          length: this.filteredRows.length,
        },
        bubbles: true,
        composed: true,
      }),
    );
  }

  // -------------------------------------------------------------- Selection

  private toggleRow(row: TableRow, checked: boolean) {
    const id = this.idForDisplayRow(row);
    const next = new Set(this.selectedIds);
    if (checked) {
      next.add(id);
    } else {
      next.delete(id);
    }
    this.selectedIds = next;
    this.emitSelection();
  }

  private toggleAll(checked: boolean) {
    const next = new Set(this.selectedIds);
    // Operate on the current page's visible rows (Data Grid semantics).
    for (const row of this.pagedRows) {
      const id = this.idForDisplayRow(row);
      if (checked) {
        next.add(id);
      } else {
        next.delete(id);
      }
    }
    this.selectedIds = next;
    this.emitSelection();
  }

  /** Map a display row back to its stable id via source identity. */
  private idForDisplayRow(row: TableRow): string | number {
    const sourceIndex = this.rows.indexOf(row);
    return this.rowId(row, sourceIndex);
  }

  private emitSelection() {
    const display = this.pagedRows;
    const selectedDisplayIndices: number[] = [];
    display.forEach((row, i) => {
      if (this.selectedIds.has(this.idForDisplayRow(row))) {
        selectedDisplayIndices.push(i);
      }
    });
    const selectedRows = this.rows.filter((row, i) => this.selectedIds.has(this.rowId(row, i)));
    this.dispatchEvent(
      new CustomEvent<TableSelectionEventDetail>("table:selection-change", {
        detail: {
          selected: selectedDisplayIndices,
          selectedIds: [...this.selectedIds],
          selectedRows,
        },
        bubbles: true,
        composed: true,
      }),
    );
  }

  // ---------------------------------------------------------- Resize/reorder

  private startResize(column: TableColumn, event: PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    const th = (event.currentTarget as HTMLElement).closest("th");
    const startWidth = th ? th.getBoundingClientRect().width : 100;
    this.resizeState = { key: column.key, startX: event.clientX, startWidth };
    window.addEventListener("pointermove", this.onResizeMove);
    window.addEventListener("pointerup", this.onResizeEnd);
  }

  private readonly onResizeMove = (event: PointerEvent) => {
    if (!this.resizeState) {
      return;
    }
    const delta = event.clientX - this.resizeState.startX;
    const width = Math.max(48, this.resizeState.startWidth + delta);
    const next = new Map(this.columnWidths);
    next.set(this.resizeState.key, `${Math.round(width)}px`);
    this.columnWidths = next;
  };

  private readonly onResizeEnd = (): void => {
    this.resizeState = null;
    window.removeEventListener("pointermove", this.onResizeMove);
    window.removeEventListener("pointerup", this.onResizeEnd);
  };

  private handleDragStart(column: TableColumn) {
    this.dragKey = column.key;
  }

  private handleDrop(target: TableColumn) {
    if (this.dragKey === null || this.dragKey === target.key) {
      this.dragKey = null;
      return;
    }
    const order = this.orderedColumns.map((c) => c.key);
    const from = order.indexOf(this.dragKey);
    const to = order.indexOf(target.key);
    if (from === -1 || to === -1) {
      this.dragKey = null;
      return;
    }
    order.splice(to, 0, order.splice(from, 1)[0]);
    this.columnOrder = order;
    this.dragKey = null;
  }

  // --------------------------------------------------------- Cell editing

  /** Whether a given column allows inline editing. */
  private isEditable(column: TableColumn): boolean {
    return column.editable === true;
  }

  /** Whether a given display row + column is the cell currently in edit mode. */
  private isEditingCell(row: TableRow, column: TableColumn): boolean {
    return (
      this.editing !== null &&
      this.editing.key === column.key &&
      this.editing.rowId === this.idForDisplayRow(row)
    );
  }

  /** Enter edit mode for a display row + column (no-op if not editable). */
  private beginEdit(row: TableRow, column: TableColumn): void {
    if (!this.isEditable(column)) {
      return;
    }
    this.editing = { rowId: this.idForDisplayRow(row), key: column.key };
    this.editorNeedsFocus = true;
  }

  /** Leave edit mode without committing. */
  private cancelEdit(): void {
    this.editing = null;
  }

  /**
   * Commit `value` to the cell currently in edit mode. Resolves the source row
   * by its stable id (so tri/filtre/pagination/identité stable are respected),
   * mutates it in place, emits `table:cell-edit`, and exits edit mode.
   */
  private commitEdit(value: string): void {
    const editing = this.editing;
    if (!editing) {
      return;
    }
    const sourceIndex = this.rows.findIndex((row, i) => this.rowId(row, i) === editing.rowId);
    this.editing = null;
    if (sourceIndex === -1) {
      return;
    }
    const row = this.rows[sourceIndex];
    const previous = row[editing.key];
    const previousText = previous === null || previous === undefined ? "" : String(previous);
    if (value === previousText) {
      // No change: skip the mutation + event but still leave edit mode.
      return;
    }
    // Mutate in place so the memoised pipeline (which holds row references)
    // reflects the change; request an update to re-derive filter/sort.
    row[editing.key] = value;
    this.requestUpdate();
    this.dispatchEvent(
      new CustomEvent<TableCellEditEventDetail>("table:cell-edit", {
        detail: { rowId: editing.rowId, field: editing.key, value, row },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleCellKeydown(event: KeyboardEvent, row: TableRow, column: TableColumn): void {
    if (!this.isEditable(column) || this.isEditingCell(row, column)) {
      return;
    }
    if (event.key === "Enter" || event.key === "F2") {
      event.preventDefault();
      this.beginEdit(row, column);
    }
  }

  private handleEditorKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      this.commitEdit((event.target as HTMLInputElement).value);
    } else if (event.key === "Escape") {
      event.preventDefault();
      this.cancelEdit();
    }
  }

  private handleEditorBlur(event: Event): void {
    // Blur commits (Escape already exited edit mode, so `editing` is null then).
    if (this.editing) {
      this.commitEdit((event.target as HTMLInputElement).value);
    }
  }

  // ----------------------------------------------------------- Public API

  /**
   * The rows in current display order: filtered and sorted (but NOT paginated).
   * Useful for export and external consumers.
   */
  get displayRows(): TableRow[] {
    // Computed fresh (not from the render-time memo) so external callers see
    // the current data even before the next render flush.
    return this.computeSortedRows(this.computeFilteredRows());
  }

  /** Programmatically set the global quick filter. */
  setQuickFilter(value: string) {
    this.quickFilter = value;
    this.pageIndex = 0;
    this.emitFilter();
  }

  /** Programmatically replace the active sort criteria. */
  setSort(criteria: TableSortCriterion[]) {
    this.applySort(criteria.map((c) => ({ ...c })));
  }

  /** Clear all filters (quick + per-column). */
  clearFilters() {
    this.quickFilter = "";
    this.columnFilters = new Map();
    this.pageIndex = 0;
    this.emitFilter();
  }

  /** The currently selected row records (across all pages). */
  getSelectedRows(): TableRow[] {
    return this.rows.filter((row, i) => this.selectedIds.has(this.rowId(row, i)));
  }

  /**
   * Serialise the current view (filtered + sorted, all pages) to a CSV string.
   * Honours the displayed column order. Values are CSV-escaped per RFC 4180.
   */
  toCsv(): string {
    const cols = this.computeOrderedColumns();
    const sortedRows = this.computeSortedRows(this.computeFilteredRows());
    const escape = (v: unknown): string => {
      const s = v === null || v === undefined ? "" : String(v);
      if (/[",\n\r]/.test(s)) {
        return `"${s.replace(/"/g, '""')}"`;
      }
      return s;
    };
    const header = cols.map((c) => escape(c.label)).join(",");
    const lines = sortedRows.map((row) => cols.map((c) => escape(row[c.key])).join(","));
    return [header, ...lines].join("\r\n");
  }

  /**
   * Export the current view to a downloaded CSV file (browser only). Respects
   * the active filter and sort. Returns the CSV string for convenience.
   *
   * @param filename Download filename (defaults to `table.csv`).
   */
  exportCsv(filename = "table.csv"): string {
    const csv = this.toCsv();
    if (typeof document !== "undefined" && typeof URL !== "undefined") {
      const blob = new Blob([`﻿${csv}`], {
        type: "text/csv;charset=utf-8;",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.style.display = "none";
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    }
    return csv;
  }

  // ------------------------------------------------------------- Rendering

  private sortOrdinal(column: TableColumn): number {
    if (this.sortCriteria.length < 2) {
      return 0;
    }
    const idx = this.sortCriteria.findIndex((c) => c.key === column.key);
    return idx === -1 ? 0 : idx + 1;
  }

  private renderSortIndicator(column: TableColumn) {
    const crit = this.sortCriteria.find((c) => c.key === column.key);
    if (!column.sortable || !crit) {
      return nothing;
    }
    const arrow = crit.direction === "desc" ? "▼" : "▲";
    const ordinal = this.sortOrdinal(column);
    return html`<span class="sort-arrow" aria-hidden="true"
      >${arrow}${ordinal ? html`<sub class="sort-ord">${ordinal}</sub>` : nothing}</span
    >`;
  }

  private renderHeaderCell(column: TableColumn) {
    const sorted = !!this.sortCriteria.find((c) => c.key === column.key);
    const classes = {
      numeric: column.numeric ?? false,
      sortable: column.sortable ?? false,
      sorted: (column.sortable ?? false) && sorted,
    };
    const width = this.columnWidths.get(column.key) ?? column.width;
    const canReorder = this.reorderable;
    return html`
      <th
        scope="col"
        role="columnheader"
        class=${classMap(classes)}
        style=${width ? styleMap({ width }) : nothing}
        aria-sort=${this.ariaSortFor(column)}
        tabindex=${column.sortable ? "0" : "-1"}
        draggable=${canReorder ? "true" : "false"}
        @dragstart=${canReorder
          ? () => {
              this.handleDragStart(column);
            }
          : nothing}
        @dragover=${canReorder
          ? (e: DragEvent) => {
              e.preventDefault();
            }
          : nothing}
        @drop=${canReorder
          ? (e: DragEvent) => {
              e.preventDefault();
              this.handleDrop(column);
            }
          : nothing}
        @click=${(e: MouseEvent) => {
          this.handleHeaderClick(column, e);
        }}
        @keydown=${(event: KeyboardEvent) => {
          this.handleHeaderKeydown(event, column);
        }}
      >
        <span class="header-content">
          <span class="header-label">${column.label}</span>
          ${this.renderSortIndicator(column)}
        </span>
        ${column.resizable
          ? html`<span
              class="resize-handle"
              aria-hidden="true"
              @pointerdown=${(e: PointerEvent) => {
                this.startResize(column, e);
              }}
              @click=${(e: Event) => {
                e.stopPropagation();
              }}
            ></span>`
          : nothing}
      </th>
    `;
  }

  private renderFilterCell(column: TableColumn) {
    if (!column.filter) {
      return html`<th class="filter-cell" aria-hidden="true"></th>`;
    }
    const current = this.columnFilters.get(column.key);
    const value = current?.value ?? "";
    const isNumeric = column.filter === "number" || column.numeric;
    const operator = current?.operator ?? (isNumeric ? "equals" : "contains");
    return html`
      <th class="filter-cell">
        <span class="filter-control">
          ${isNumeric
            ? html`<select
                class="filter-op"
                aria-label="${column.label} filter operator"
                .value=${operator}
                @change=${(e: Event) => {
                  this.handleColumnFilterOperator(column, e);
                }}
              >
                <option value="equals" ?selected=${operator === "equals"}>=</option>
                <option value="gt" ?selected=${operator === "gt"}>&gt;</option>
                <option value="lt" ?selected=${operator === "lt"}>&lt;</option>
                <option value="gte" ?selected=${operator === "gte"}>&ge;</option>
                <option value="lte" ?selected=${operator === "lte"}>&le;</option>
              </select>`
            : nothing}
          <input
            type=${isNumeric ? "number" : "text"}
            class="filter-input"
            aria-label="Filter ${column.label}"
            placeholder="Filter"
            .value=${value}
            @input=${(e: Event) => {
              this.handleColumnFilterValue(column, e);
            }}
          />
        </span>
      </th>
    `;
  }

  private renderSelectAll(rows: TableRow[]) {
    const total = rows.length;
    let count = 0;
    for (const row of rows) {
      if (this.selectedIds.has(this.idForDisplayRow(row))) {
        count++;
      }
    }
    const allChecked = total > 0 && count === total;
    const indeterminate = count > 0 && count < total;
    return html`
      <th scope="col" role="columnheader" class="checkbox-cell">
        <input
          type="checkbox"
          class="md-checkbox"
          aria-label="Select all rows"
          .checked=${allChecked}
          .indeterminate=${indeterminate}
          @change=${(event: Event) => {
            this.toggleAll((event.target as HTMLInputElement).checked);
          }}
        />
      </th>
    `;
  }

  private renderBodyRow(row: TableRow, _index: number) {
    const selected = this.selectedIds.has(this.idForDisplayRow(row));
    return html`
      <tr role="row" class=${classMap({ selected: selected })}>
        ${this.selectable
          ? html`
              <td role="cell" class="checkbox-cell">
                <input
                  type="checkbox"
                  class="md-checkbox"
                  aria-label="Select row"
                  .checked=${selected}
                  @change=${(event: Event) => {
                    this.toggleRow(row, (event.target as HTMLInputElement).checked);
                  }}
                />
              </td>
            `
          : nothing}
        ${this.orderedColumns.map((column) => {
          const value = row[column.key];
          const text = value === null || value === undefined ? "" : String(value);
          const width = this.columnWidths.get(column.key) ?? column.width;
          const editable = this.isEditable(column);
          const isEditing = editable && this.isEditingCell(row, column);
          return html`
            <td
              role="cell"
              class=${classMap({
                numeric: column.numeric ?? false,
                editable,
                editing: isEditing,
              })}
              style=${width ? styleMap({ width }) : nothing}
              tabindex=${editable && !isEditing ? "0" : "-1"}
              aria-readonly=${editable ? "false" : nothing}
              @dblclick=${editable
                ? () => {
                    this.beginEdit(row, column);
                  }
                : nothing}
              @keydown=${editable
                ? (e: KeyboardEvent) => {
                    this.handleCellKeydown(e, row, column);
                  }
                : nothing}
            >
              ${isEditing
                ? html`<input
                    class="cell-editor"
                    type=${column.numeric ? "number" : "text"}
                    aria-label=${`Edit ${column.label}`}
                    .value=${text}
                    @keydown=${(e: KeyboardEvent) => {
                      this.handleEditorKeydown(e);
                    }}
                    @blur=${(e: Event) => {
                      this.handleEditorBlur(e);
                    }}
                  />`
                : text}
            </td>
          `;
        })}
      </tr>
    `;
  }

  private renderPaginator(): TemplateResult {
    const length = this.filteredRows.length;
    const start = length === 0 ? 0 : this.pageIndex * this.pageSize + 1;
    const end = length === 0 ? 0 : Math.min((this.pageIndex + 1) * this.pageSize, length);
    const hasPrev = this.pageIndex >= 1 && length > 0;
    const hasNext = this.pageIndex < this.pageCount - 1 && length > 0;
    const navButton = (label: string, disabled: boolean, handler: () => void, path: string) => html`
      <button
        type="button"
        class="icon-button"
        aria-label=${label}
        ?disabled=${disabled}
        @click=${handler}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d=${path}></path></svg>
      </button>
    `;
    return html`
      <div class="paginator" role="group" aria-label="Pagination">
        <div class="page-size">
          <span class="page-size-label" id="page-size-label">Rows per page:</span>
          <span class="select-wrapper">
            <select
              class="select"
              aria-labelledby="page-size-label"
              .value=${String(this.pageSize)}
              @change=${(e: Event) => {
                this.handlePageSizeChange(e);
              }}
            >
              ${this.rowsPerPageOptions.map(
                (size) => html`
                  <option value=${size} ?selected=${size === this.pageSize}>${size}</option>
                `,
              )}
            </select>
            <span class="select-arrow" aria-hidden="true">
              <svg viewBox="0 0 24 24"><path d="M7 10l5 5 5-5z"></path></svg>
            </span>
          </span>
        </div>
        <div class="range-label" aria-live="polite">${start}–${end} of ${length}</div>
        <div class="actions">
          ${navButton(
            "First page",
            !hasPrev,
            () => {
              this.setPageIndex(0);
            },
            "M18.41 16.59 13.82 12l4.59-4.59L17 6l-6 6 6 6zM6 6h2v12H6z",
          )}
          ${navButton(
            "Previous page",
            !hasPrev,
            () => {
              this.setPageIndex(this.pageIndex - 1);
            },
            "M15.41 7.41 14 6l-6 6 6 6 1.41-1.41L10.83 12z",
          )}
          ${navButton(
            "Next page",
            !hasNext,
            () => {
              this.setPageIndex(this.pageIndex + 1);
            },
            "M10 6 8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z",
          )}
          ${navButton(
            "Last page",
            !hasNext,
            () => {
              this.setPageIndex(this.pageCount - 1);
            },
            "M5.59 7.41 10.18 12l-4.59 4.59L7 18l6-6-6-6zM16 6h2v12h-2z",
          )}
        </div>
      </div>
    `;
  }

  private renderToolbar(): TemplateResult {
    return html`
      <div class="toolbar">
        <span class="search-field">
          <svg class="search-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M15.5 14h-.79l-.28-.27a6.5 6.5 0 1 0-.7.7l.27.28v.79l5 4.99L20.49 19zm-6 0A4.5 4.5 0 1 1 14 9.5 4.49 4.49 0 0 1 9.5 14z"
            ></path>
          </svg>
          <input
            type="search"
            class="search-input"
            aria-label="Search all columns"
            placeholder="Search"
            .value=${this.quickFilter}
            @input=${(e: Event) => {
              this.handleQuickFilter(e);
            }}
          />
        </span>
      </div>
    `;
  }

  protected override render() {
    const cols = this.orderedColumns;
    const rows = this.pagedRows;
    const colCount = cols.length + (this.selectable ? 1 : 0);
    const showFilterRow = this.filterable && cols.some((c) => !!c.filter);
    return html`
      <div class="container">
        ${this.filterable ? this.renderToolbar() : nothing}
        <div class="table-scroll">
          <table role="grid">
            <thead>
              <tr role="row">
                ${this.selectable ? this.renderSelectAll(rows) : nothing}
                ${cols.map((column) => this.renderHeaderCell(column))}
              </tr>
              ${showFilterRow
                ? html`<tr role="row" class="filter-row">
                    ${this.selectable
                      ? html`<th class="checkbox-cell" aria-hidden="true"></th>`
                      : nothing}
                    ${cols.map((column) => this.renderFilterCell(column))}
                  </tr>`
                : nothing}
            </thead>
            <tbody>
              ${rows.length === 0
                ? html`
                    <tr role="row" class="empty">
                      <td role="cell" colspan=${colCount} class="empty-cell">No data</td>
                    </tr>
                  `
                : repeat(
                    rows,
                    (row, index) => this.idForDisplayRow(row) ?? index,
                    (row, index) => this.renderBodyRow(row, index),
                  )}
            </tbody>
          </table>
        </div>
        ${this.paginated ? this.renderPaginator() : nothing}
      </div>
    `;
  }
}
