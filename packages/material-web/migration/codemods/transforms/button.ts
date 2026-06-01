/**
 * transforms/button.ts — transform DÉDIÉ Button / IconButton / Fab (variant-aware).
 *
 * Conforme à 00-CONVENTIONS.md §3 (Button variant-dépendant) :
 *   variant="contained"|absent -> MdFilledButton
 *   variant="outlined"         -> MdOutlinedButton
 *   variant="text"             -> MdTextButton
 *   variant="elevated"|"tonal" -> MdElevatedButton / MdFilledTonalButton
 * Gère startIcon/endIcon -> slots (§4) et retire la prop `variant`.
 *
 * Usage :
 *   bunx jscodeshift -t transforms/button.ts --parser=tsx <fichiers...>
 */
import type { API, FileInfo, Options } from "jscodeshift";
import { runEngine } from "../lib/engine";

export default function transformer(file: FileInfo, api: API, options: Options): string {
  return runEngine(file, api, {
    ...options,
    only: new Set(["Button", "IconButton", "Fab"]),
  });
}

export const parser = "tsx";
