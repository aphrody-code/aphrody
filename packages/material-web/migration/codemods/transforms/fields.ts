/**
 * transforms/fields.ts — transform DÉDIÉ champs : TextField / Select / NativeSelect.
 *
 * Conforme à 00-CONVENTIONS.md §3 (TextField/Select variant-dépendants) et §4
 * (controlled : onChange(e,val) -> e.target.value) :
 *   TextField variant="filled"|absent -> MdFilledTextField
 *   TextField variant="outlined"       -> MdOutlinedTextField
 *   Select    filled/outlined          -> MdFilledSelect / MdOutlinedSelect
 * Inclut MenuItem -> MdMenuItem pour le contenu des Select (cf. §3).
 * Pose un MIGRATION-TODO quand onChange a une signature (e, value).
 *
 * Usage :
 *   bunx jscodeshift -t transforms/fields.ts --parser=tsx <fichiers...>
 */
import type { API, FileInfo, Options } from "jscodeshift";
import { runEngine } from "../lib/engine";

export default function transformer(file: FileInfo, api: API, options: Options): string {
  return runEngine(file, api, {
    ...options,
    only: new Set(["TextField", "Select", "NativeSelect", "MenuItem"]),
  });
}

export const parser = "tsx";
