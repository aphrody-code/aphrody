/**
 * transforms/mui-imports.ts — réécriture GÉNÉRIQUE des imports + JSX.
 *
 * Traite TOUS les composants connus du mapping (cf. 00-CONVENTIONS.md §3) :
 *   - imports nommés    `import {Button} from '@mui/material'`
 *   - imports default   `import Button from '@mui/material/Button'`
 *   - alias             `import {Button as B} from '@mui/material'`
 * Réécrit vers `@aphrody/m3-react` (§2) en choisissant le wrapper variant-aware.
 *
 * Usage :
 *   bunx jscodeshift -t transforms/mui-imports.ts --parser=tsx <fichiers...>
 */
import type { API, FileInfo, Options } from "jscodeshift";
import { runEngine } from "../lib/engine";

export default function transformer(file: FileInfo, api: API, options: Options): string {
  // pas de `only` -> traite tous les composants mappés
  return runEngine(file, api, options);
}

export const parser = "tsx";
