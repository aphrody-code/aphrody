<!-- SPDX-License-Identifier: Apache-2.0 -->

# Migration references — third-party source material

These files describe the **MUI source idioms** that the `migrate-mui` skill
migrates _away from_. They are not part of the M3 system; they are vendored as a
faithful, attributed reference so the migration knows exactly what the source
code looks like (the `sx`/`styled`/`theme` styling wall is the real migration
cost — see `migration/10-case-study-rpbey.md`).

## Contents

| Path                | What                                                                                                                                      | Source                                                       |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `mui-docs-index.md` | Material UI documentation index (the `llms.txt` link map: every component + customization page).                                          | `https://mui.com/material-ui/llms.txt` (fetched 2026-05-29). |
| `mui-upstream/`     | The official Material UI agent skills (styling / theming / nextjs / tailwind) verbatim. The canonical detail is each skill's `AGENTS.md`. | `github.com/mui/material-ui` `skills/` at commit `861f7cc`.  |

## Attribution and licence

The vendored MUI material (`mui-docs-index.md`, `mui-upstream/**`) is
Copyright (c) 2014 Call-Em-All and the MUI contributors, distributed under the
**MIT License** — its full text is preserved at `mui-upstream/UPSTREAM-LICENSE`.
It is reproduced here unmodified under that licence; it is **not** relicensed.
The Apache-2.0 SPDX header on this NOTICE applies only to this file and the
surrounding `material-design` plugin, not to the vendored MUI content.

## How the skill uses these

- `mui-docs-index.md` resolves "what is MUI `<X>`?" to the upstream doc page, so
  the migration maps each MUI component to its M3 counterpart (see
  `migration/mui-m3-map.json` for the consolidated mapping).
- `mui-upstream/material-ui-styling` and `material-ui-theming` document the
  exact `sx` / `styled` / `createTheme` patterns to detect and rewrite to
  Tailwind on the host + `--md-sys-*` tokens (cf. `m3/no-sx-prop`,
  `migration/06-tailwind-material-web.md`).
- `mui-upstream/material-ui-tailwind` informs the Tailwind interop layer.
- `mui-upstream/material-ui-nextjs` informs framework wiring during a port.
