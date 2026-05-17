<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/plugin-package-contract

In-tree mirror of [openclaw/openclaw](https://github.com/openclaw/openclaw)'s
`@openclaw/plugin-package-contract` package, vendored at
`C:/worktree/openclaw/packages/plugin-package-contract/src/index.ts` as of 2026-05-17.

The contract describes the JSON shape an external plugin's `package.json`
must expose under its `openclaw` block so the host runtime can:

1. Validate that the plugin declares the required compatibility fields.
2. Normalise the declared compatibility into a stable
   `ExternalPluginCompatibility` record (used by the registry + gateway).
3. Return per-field validation issues for installer UX.

## Exports

| Symbol | Purpose |
|---|---|
| `ExternalPluginCompatibility` | Stable shape returned by `normalizeExternalPluginCompatibility`. |
| `ExternalPluginValidationIssue` | `{ fieldPath, message }` pair surfaced by the validator. |
| `ExternalCodePluginValidationResult` | Combined `{ compatibility, issues }` payload. |
| `EXTERNAL_CODE_PLUGIN_REQUIRED_FIELD_PATHS` | Tuple of dot-paths required for a code plugin. |
| `normalizeExternalPluginCompatibility(packageJson)` | Pure normaliser. |
| `listMissingExternalCodePluginFieldPaths(packageJson)` | Returns the missing required paths. |
| `validateExternalCodePluginPackageJson(packageJson)` | High-level validator. |

## Licence

- Upstream licence: MIT, Copyright (c) 2025 Peter Steinberger.
- Aphrody redistribution: Apache-2.0 with upstream attribution preserved
  in the source banner of `index.ts` (see SPDX header).

## Refresh

```bash
bun run scripts/plugin-contract-port.ts
```

Pass `--source-oc=<path>` to override the upstream clone location
(defaults to `C:/worktree/openclaw`).

Use `--check` in CI to fail-fast when the upstream clone is missing.

## Aphrody consumers

- `crates/aphrody-mcp` — plugin registry compatibility gate.
- `crates/aphrody-gateway` — provider plugin handshake.
- `packages/agui-adapter` (Wave-2) — AGUI runtime plugin loader.
