/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

import { CSSResultOrNative } from "lit";
import { customElement } from "lit/decorators.js";

import { Table } from "./internal/table.js";
import { styles } from "./internal/table-styles.js";

declare global {
  interface HTMLElementTagNameMap {
    "md-table": MdTable;
  }
}

/**
 * @summary Data tables display information in a grid-like format of rows and
 * columns, with sortable headers, filtering, pagination, row selection and
 * CSV export — a Material 3 take on the MUI X Data Grid (Community) feature set.
 *
 * @description
 * Data-driven: assign `.columns` and `.rows` and the element renders a real
 * `<table>` (`role="grid"`). Clicking a sortable header cycles ascending →
 * descending → unsorted and fires `table:sort`; with `multi-sort`, Shift+click
 * appends additional sort criteria. Set `selectable` for a leading checkbox
 * column that fires `table:selection-change`. Set `filterable` for a global
 * quick-search field plus per-column filters (`filter: 'text' | 'number'` on a
 * column), firing `table:filter`. Set `paginated` for an integrated paginator
 * (`page-size`, `page-index`, `.rowsPerPageOptions`), firing `table:page`. Set
 * `reorderable` to drag-reorder columns, and `resizable` on a column to drag its
 * trailing edge. Call `exportCsv()` to download the current (filtered + sorted)
 * view as CSV.
 *
 * The display pipeline is filter → sort → paginate; `displayRows`,
 * `getSelectedRows()` and `exportCsv()` all reflect it.
 *
 * ```html
 * <md-table selectable filterable paginated multi-sort></md-table>
 * <script>
 *   const t = document.querySelector('md-table');
 *   t.columns = [
 *     {key: 'name', label: 'Dessert', filter: 'text'},
 *     {key: 'calories', label: 'Calories', numeric: true, sortable: true,
 *      filter: 'number', resizable: true},
 *   ];
 *   t.rows = [{name: 'Frozen yogurt', calories: 159}];
 * </script>
 * ```
 *
 * @final
 * @suppress {visibility}
 */
@customElement("md-table")
export class MdTable extends Table {
  static override styles: CSSResultOrNative[] = [styles];
}
