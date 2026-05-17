# material-web — Material Web Components 3 reference (excluded)

This directory is **intentionally gitignored** (see `.gitignore` §21).

The full material-web monorepo (~20 MB) is the official Material Design 3 Web Components by Google. We don't vendor it — we consume the published npm package directly.

## Production consumption

aphrody's `packages/ui` (shadcn fork → M3 wrappers) imports from `@material/web`:

```ts
import { MdFilledButton } from '@material/web/button/filled-button.js';
```

Add to your project:

```bash
bun add @material/web
```

## Re-clone (only for spec study)

```bash
gh repo clone material-components/material-web packages/material-web
```

Useful only if you need to read upstream source for behavior cross-checks. `docs/research/SHADCN_M3_MAPPING.md` already captures the integration plan.

Upstream : <https://github.com/material-components/material-web>
Spec : <https://m3.material.io/>
