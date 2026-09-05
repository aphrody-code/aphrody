/**
 * transforms/icons.ts — codemod DÉDIÉ aux icônes : @mui/icons-material -> md-icon.
 *
 * Transforme :
 *   import CloseIcon from '@mui/icons-material/Close';
 *   import { Delete, EmojiEvents } from '@mui/icons-material';
 *   ...
 *   <CloseIcon fontSize="small" />   ->   <md-icon>close</md-icon>
 *   <Delete />                       ->   <md-icon>delete</md-icon>
 *
 * Stratégie (cf. lib/icon-names.ts) :
 *   - glyphe Material Symbols valide -> <md-icon>glyph</md-icon> + import d'effet
 *     de bord '@aphrody/material-web/icon/icon.js'.
 *   - logo de marque (GitHub, X…) absent de Material Symbols -> ÉLÉMENT INCHANGÉ
 *     + MIGRATION-TODO (garder l'icône en SVG / set de marque dédié).
 *   - conversion non validée -> élément inchangé + MIGRATION-TODO (avec le snake
 *     case deviné pour aider).
 *
 * Idempotent. Les props MUI de l'icône (fontSize, color, sx…) sont retirées avec
 * un TODO si non triviales (fontSize -> --md-icon-size, color -> currentColor).
 *
 * Usage :
 *   bunx jscodeshift -t transforms/icons.ts --parser=tsx --extensions=tsx 'src/**\/*.tsx'
 * (à lancer APRÈS l'orchestrateur, ou seul pour ne migrer que les icônes.)
 */
import type { API, ASTPath, FileInfo, Options } from "jscodeshift";
import { resolveMuiIcon } from "../lib/icon-names";
import { addMigrationTodo, flushMigrationTodos } from "../lib/jsx-helpers";

const MUI_ICONS_PKG = "@mui/icons-material";
const MD_ICON_SIDE_EFFECT = "@aphrody/material-web/icon/icon.js";

export const parser = "tsx";

export default function transformer(file: FileInfo, api: API, _options: Options): string {
  const j = api.jscodeshift;
  const root = j(file.source);

  // 1) Collecte : localName JSX -> nom d'icône MUI canonique.
  const bindings = new Map<string, string>();
  const importPathsToDrop: ASTPath<any>[] = [];

  root.find(j.ImportDeclaration).forEach((p) => {
    const src = p.node.source.value;
    if (typeof src !== "string") return;
    if (src === MUI_ICONS_PKG) {
      // import { Close, Delete as Trash } from '@mui/icons-material'
      for (const spec of p.node.specifiers || []) {
        if (spec.type === "ImportSpecifier") {
          const imported = String((spec.imported as any).name);
          const local = String(spec.local?.name || imported);
          bindings.set(local, imported);
        }
      }
      importPathsToDrop.push(p);
    } else if (src.startsWith(MUI_ICONS_PKG + "/")) {
      // import CloseIcon from '@mui/icons-material/Close'
      const iconName = src.slice(MUI_ICONS_PKG.length + 1).split("/")[0];
      for (const spec of p.node.specifiers || []) {
        if (spec.type === "ImportDefaultSpecifier" && spec.local?.name) {
          bindings.set(String(spec.local.name), iconName);
        }
      }
      importPathsToDrop.push(p);
    }
  });

  if (bindings.size === 0) return file.source;

  // 2) Réécriture des usages JSX.
  let needSideEffect = false;
  const usedLocals = new Set<string>();
  const keptLocals = new Set<string>(); // brand/unknown -> on garde l'import

  root.find(j.JSXElement).forEach((path: ASTPath<any>) => {
    const open = path.node.openingElement;
    if (open.name.type !== "JSXIdentifier") return;
    const local = open.name.name;
    const muiName = bindings.get(local);
    if (!muiName) return;

    usedLocals.add(local);
    const res = resolveMuiIcon(muiName);

    if (res.kind === "symbol" && res.glyph) {
      // <md-icon>glyph</md-icon>
      const hadProps = (open.attributes || []).some(
        (a: any) => a.type === "JSXAttribute" || a.type === "JSXSpreadAttribute",
      );
      const el = path.node;
      el.openingElement.name = j.jsxIdentifier("md-icon");
      el.openingElement.attributes = [];
      el.openingElement.selfClosing = false;
      el.closingElement = j.jsxClosingElement(j.jsxIdentifier("md-icon"));
      el.children = [j.jsxText(res.glyph)];
      needSideEffect = true;
      if (hadProps) {
        addMigrationTodo(
          j,
          path,
          `icone ${muiName}: props MUI (fontSize/color/sx) retirees -> piloter via --md-icon-size / --md-icon-fill / currentColor.`,
        );
      }
    } else if (res.kind === "brand") {
      keptLocals.add(local);
      addMigrationTodo(
        j,
        path,
        `icone ${muiName}: logo de marque absent de Material Symbols -> garder en SVG (set de marque dedie). Slug suggere: ${res.guess}.`,
      );
    } else {
      keptLocals.add(local);
      addMigrationTodo(
        j,
        path,
        `icone ${muiName}: nom Material Symbols non valide (devine: "${res.guess}") -> verifier sur fonts.google.com/icons puis <md-icon>nom</md-icon>.`,
      );
    }
  });

  flushMigrationTodos(j, root);

  // 3) Nettoyage des imports : retirer les specifiers d'icônes effectivement
  //    converties (symbol) ; conserver ceux gardés (brand/unknown).
  for (const p of importPathsToDrop) {
    const decl = p.node;
    decl.specifiers = (decl.specifiers || []).filter((spec: any) => {
      const local =
        spec.type === "ImportSpecifier"
          ? String(spec.local?.name || (spec.imported as any).name)
          : String(spec.local?.name || "");
      // garde si non utilisé (laisse tel quel) OU gardé (brand/unknown)
      if (!usedLocals.has(local)) return true;
      return keptLocals.has(local);
    });
    if ((decl.specifiers || []).length === 0) j(p).remove();
  }

  // 4) Import d'effet de bord md-icon (après le prologue de directives).
  if (needSideEffect) {
    const body = root.get().node.program.body;
    const alreadyImported = root
      .find(j.ImportDeclaration)
      .some((p: ASTPath<any>) => p.node.source.value === MD_ICON_SIDE_EFFECT);
    if (!alreadyImported) {
      let insertAt = 0;
      while (insertAt < body.length) {
        const st = body[insertAt];
        const isDirective =
          st?.type === "ExpressionStatement" &&
          (st.expression?.type === "StringLiteral" ||
            (st.expression?.type === "Literal" && typeof st.expression.value === "string"));
        if (!isDirective) break;
        insertAt++;
      }
      body.splice(insertAt, 0, j.importDeclaration([], j.stringLiteral(MD_ICON_SIDE_EFFECT)));
    }
  }

  const out = root.toSource({ quote: "single" });
  return out.replace(/^(\s*(['"])use (?:client|strict)\2);;/gm, "$1;");
}
