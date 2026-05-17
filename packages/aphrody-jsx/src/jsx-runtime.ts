/** @license SPDX-License-Identifier: Apache-2.0 */

// JSX automatic-runtime entry. When tsconfig sets
//   "jsx": "react-jsx",
//   "jsxImportSource": "@aphrody/jsx"
// the compiler emits `import { jsx, jsxs, Fragment } from "@aphrody/jsx/jsx-runtime"`.
// We forward those imports straight to React's own automatic runtime — the
// reconciler operates on the resulting React elements, so no rewrite is
// needed at the JSX layer.

export { Fragment, jsx, jsxs } from "react/jsx-runtime";
