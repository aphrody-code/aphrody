/**
 * lib/engine.ts — moteur de transformation partagé.
 *
 * Conforme à 00-CONVENTIONS.md §2/§3/§4. Implémente, en un seul passage AST :
 *  1. collecte des local-names MUI importés (nommés, default, alias) depuis
 *     `@mui/material` et `@mui/material/<Composant>`.
 *  2. réécriture des éléments JSX -> wrappers `@aphrody-code/m3-react` (variant-aware
 *     pour Button/TextField/Select).
 *  3. réécriture des props (startIcon/endIcon -> slots, onChange (e,val), sx,
 *     props sans équivalent) avec marqueurs MIGRATION-TODO.
 *  4. réécriture/insertion des imports vers `@aphrody-code/m3-react`.
 *
 * Les transforms de `transforms/` ne font que paramétrer ce moteur (quels
 * composants traiter) pour rester focalisés (Button, champs, imports, orchestrateur).
 */
import type {
  API,
  Collection,
  FileInfo,
  JSXElement,
  JSXOpeningElement,
  Options,
} from "jscodeshift";
import {
  GAP_COMPONENTS,
  LAYOUT_COMPONENTS,
  M3_PKG,
  MUI_PKGS,
  PROP_DROP_WITH_TODO,
  RENAME_CHILDREN,
  SIMPLE_COMPONENTS,
  SLOTTED_CHILDREN,
  VARIANT_COMPONENTS,
} from "./mapping";
import {
  addMigrationTodo,
  attrStringValue,
  findAttr,
  flushMigrationTodos,
  jsxName,
  makeSlotAttr,
  onChangeUsesSecondArg,
  removeAttr,
  renameElement,
} from "./jsx-helpers";

export interface EngineOptions {
  /** Limite le traitement à ce sous-ensemble de composants MUI (par leur nom MUI). */
  only?: Set<string>;
}

/** Info sur un import MUI repéré : localName (tel qu'utilisé) -> composant MUI canonique. */
type MuiBindings = Map<string, string>;

/** true si la source d'import est MUI (package racine ou sous-chemin). */
function isMuiSource(src: string): { mui: boolean; subComponent?: string } {
  for (const pkg of MUI_PKGS) {
    if (src === pkg) return { mui: true };
    if (src.startsWith(pkg + "/")) {
      const rest = src.slice(pkg.length + 1);
      // @mui/material/Button -> "Button" ; @mui/material/styles -> ignore
      const seg = rest.split("/")[0];
      if (/^[A-Z]/.test(seg)) return { mui: true, subComponent: seg };
    }
  }
  return { mui: false };
}

/** Résout le wrapper cible pour un composant MUI donné + son élément JSX. */
function resolveWrapper(
  muiComp: string,
  open: JSXOpeningElement | null,
): {
  kind: "wrapper" | "layout" | "gap" | "unknown";
  name?: string;
  note?: string;
} {
  if (VARIANT_COMPONENTS[muiComp]) {
    const vm = VARIANT_COMPONENTS[muiComp];
    let variant: string | null = null;
    if (open) {
      const a = findAttr(open, vm.prop);
      if (a) variant = attrStringValue(a);
    }
    const name = (variant && vm.byValue[variant]) || vm.default;
    return { kind: "wrapper", name };
  }
  if (SIMPLE_COMPONENTS[muiComp]) return { kind: "wrapper", name: SIMPLE_COMPONENTS[muiComp] };
  if (RENAME_CHILDREN[muiComp]) return { kind: "wrapper", name: RENAME_CHILDREN[muiComp] };
  if (LAYOUT_COMPONENTS[muiComp]) {
    const t = LAYOUT_COMPONENTS[muiComp];
    return t.startsWith("Md") ? { kind: "wrapper", name: t } : { kind: "layout", name: t };
  }
  if (SLOTTED_CHILDREN[muiComp]) {
    const r = SLOTTED_CHILDREN[muiComp];
    return { kind: "layout", name: r.replaceWith, note: muiComp };
  }
  if (GAP_COMPONENTS[muiComp]) return { kind: "gap", note: GAP_COMPONENTS[muiComp] };
  return { kind: "unknown" };
}

/**
 * Traite les props d'un élément JSX selon §4. Mute l'élément + ajoute des
 * enfants <md-icon> slottés pour startIcon/endIcon.
 */
function transformProps(j: API["jscodeshift"], el: JSXElement, path: any): void {
  const open = el.openingElement;

  // startIcon / endIcon -> <md-icon slot="..."> (§4 Icônes)
  for (const [prop, slot] of [
    ["startIcon", "icon"],
    ["endIcon", "icon-end"], // md-button : un seul slot "icon" ; on documente endIcon
  ] as const) {
    const a = findAttr(open, prop);
    if (!a) continue;
    // On extrait le contenu de l'icône (souvent <Icon/> dans une expr container)
    const v = a.value;
    let iconExpr: any = null;
    if (v && v.type === "JSXExpressionContainer") iconExpr = v.expression;
    removeAttr(open, prop);
    if (iconExpr && iconExpr.type !== "JSXEmptyExpression") {
      const slotName = slot === "icon-end" ? "icon" : "icon";
      const iconWrap = j.jsxElement(
        j.jsxOpeningElement(j.jsxIdentifier("md-icon"), [makeSlotAttr(j, slotName)], false),
        j.jsxClosingElement(j.jsxIdentifier("md-icon")),
        [j.jsxExpressionContainer(iconExpr)],
      );
      el.children = el.children || [];
      el.children.unshift(iconWrap as any);
    }
    if (prop === "endIcon") {
      addMigrationTodo(
        j,
        path,
        'endIcon -> md-button n-a qu-un slot "icon" : repositionner manuellement (slot trailing).',
      );
    }
  }

  // onChange (e, value) -> e.target.value (§4 controlled). On pose un TODO si 2e arg.
  const onChange = findAttr(open, "onChange");
  if (onChange && onChangeUsesSecondArg(onChange)) {
    addMigrationTodo(
      j,
      path,
      "onChange(e, value) MUI -> material-web emet (input/change) natifs : lire e.target.value (le 2e parametre disparait).",
    );
  }

  // sx + props sans equivalent -> retirer + TODO (§4 sx/className).
  for (const prop of PROP_DROP_WITH_TODO) {
    const a = findAttr(open, prop);
    if (!a) continue;
    if (prop === "sx") {
      addMigrationTodo(
        j,
        path,
        "prop `sx` retiree : convertir en classes Tailwind (host/layout) + tokens --md-sys-* (le shadow DOM n-est pas atteignable par Tailwind, cf. §6).",
      );
    } else {
      addMigrationTodo(
        j,
        path,
        `prop \`${prop}\` retiree : pas d-equivalent direct md (gerer via tokens/density/theme).`,
      );
    }
    removeAttr(open, prop);
  }

  // variant : consommee par le choix du wrapper -> on la retire pour les variant-comps.
  // (effectue par le caller qui sait si c-est un variant-comp)
}

/** Point d'entree du moteur, appele par chaque transform. */
export function runEngine(file: FileInfo, api: API, options: Options & EngineOptions): string {
  const j = api.jscodeshift;
  const root = j(file.source);
  const only = options.only;

  // 1) Collecte des bindings MUI (nommes, default, alias).
  //    On NE supprime PAS encore les imports : seuls les composants
  //    EFFECTIVEMENT migrés (pas les gaps qui restent en JSX) verront leur
  //    specifier retiré en fin de passe -> évite les références orphelines.
  const bindings: MuiBindings = new Map();
  const muiImportPaths: any[] = [];

  root.find(j.ImportDeclaration).forEach((p) => {
    const src = p.node.source.value;
    if (typeof src !== "string") return;
    const { mui, subComponent } = isMuiSource(src);
    if (!mui) return;

    let touched = false;
    for (const spec of p.node.specifiers || []) {
      if (spec.type === "ImportSpecifier") {
        // import { Button as B } from '@mui/material'
        // `imported.name` est `string | JSXIdentifier` côté ast-types :
        // on normalise en string.
        const imported = String((spec.imported as any).name);
        const local = String(spec.local?.name || imported);
        if (!only || only.has(imported)) {
          bindings.set(local, imported);
          touched = true;
        }
      } else if (spec.type === "ImportDefaultSpecifier" && subComponent) {
        // import Button from '@mui/material/Button'
        const local = spec.local?.name ? String(spec.local.name) : undefined;
        if (local && (!only || only.has(subComponent))) {
          bindings.set(local, subComponent);
          touched = true;
        }
      }
    }
    if (touched) muiImportPaths.push(p);
  });

  if (bindings.size === 0) return file.source; // rien à faire

  // local-names réellement migrés (à retirer des imports MUI en fin de passe).
  const migratedLocals = new Set<string>();

  // 2) Réécriture des éléments JSX.
  const neededImports = new Set<string>(); // wrappers @aphrody-code/m3-react à importer
  let needMdIconImport = false;

  root.find(j.JSXElement).forEach((path) => {
    const el = path.node as JSXElement;
    const name = jsxName(el.openingElement);
    if (!name) return;
    const muiComp = bindings.get(name);
    if (!muiComp) return;

    const resolved = resolveWrapper(muiComp, el.openingElement);

    if (resolved.kind === "gap" || resolved.kind === "unknown") {
      // gap/unknown : on NE transforme PAS et on GARDE l'import MUI (le JSX
      // reste référencé). Marqueur pour traitement manuel.
      if (resolved.kind === "gap") {
        addMigrationTodo(
          j,
          path,
          `${muiComp} : ${resolved.note}. Aucune transformation auto (cf. 05-gap-analysis.md).`,
        );
      }
      return;
    }

    migratedLocals.add(name);

    // variant-comp : retirer la prop variant (consommee par le wrapper).
    if (VARIANT_COMPONENTS[muiComp]) {
      removeAttr(el.openingElement, VARIANT_COMPONENTS[muiComp].prop);
    }

    // props communes (icons, onChange, sx...)
    transformProps(j, el, path);

    if (resolved.kind === "layout") {
      // composant layout MUI -> <div> (ou <div slot> pour slotted children)
      const slotRule = SLOTTED_CHILDREN[muiComp];
      renameElement(el, resolved.name!);
      if (slotRule) {
        el.openingElement.attributes = el.openingElement.attributes || [];
        el.openingElement.attributes.unshift(makeSlotAttr(j, slotRule.slot));
        addMigrationTodo(
          j,
          path,
          `${muiComp} -> <div slot="${slotRule.slot}"> dans <${slotRule.parent}> : verifier l-imbrication (slot direct).`,
        );
      } else {
        addMigrationTodo(
          j,
          path,
          `${muiComp} (layout) -> <div> : reporter spacing/direction/grid sur des utilitaires Tailwind (cf. §6).`,
        );
      }
      if (needMdIconUsed(el)) needMdIconImport = true;
      return;
    }

    // wrapper md
    renameElement(el, resolved.name!);
    neededImports.add(resolved.name!);
    if (needMdIconUsed(el)) needMdIconImport = true;
  });

  // 2bis) Insertion différée des marqueurs MIGRATION-TODO (cas enfant JSX),
  // après la traversée pour éviter les décalages d'index.
  flushMigrationTodos(j, root);

  // 3) Nettoyage des imports MUI : retire SEULEMENT les specifiers migrés.
  //    Si un import n'a plus aucun specifier, on supprime la déclaration ;
  //    sinon on la conserve (ex : gap component encore référencé).
  for (const p of muiImportPaths) {
    const decl = p.node;
    decl.specifiers = (decl.specifiers || []).filter((spec: any) => {
      const local =
        spec.type === "ImportSpecifier" ? spec.local?.name || spec.imported.name : spec.local?.name;
      return !local || !migratedLocals.has(local);
    });
    if ((decl.specifiers || []).length === 0) j(p).remove();
  }

  // 4) Insertion de l'import @aphrody-code/m3-react.
  if (neededImports.size > 0 || needMdIconImport) {
    const body = root.get().node.program.body;
    // Insère APRÈS le prologue de directives ("use client" / "use strict") :
    // unshift en body[0] casse l'ordre (la directive doit rester en tête) et
    // fait émettre un `;;` parasite par recast. cf. 10-case-study-rpbey.md.
    const insertAt = directivePrologueLength(body);
    const toInsert: any[] = [];
    if (needMdIconImport) {
      // md-icon est utilisé en custom-element brut : import d'effet de bord.
      toInsert.push(
        j.importDeclaration([], j.stringLiteral("@aphrody-code/material-web/icon/icon.js")),
      );
    }
    const specs = [...neededImports]
      .sort()
      .map((n) => j.importSpecifier(j.jsxIdentifier(n) as any));
    if (specs.length > 0) {
      toInsert.push(j.importDeclaration(specs, j.stringLiteral(M3_PKG)));
    }
    body.splice(insertAt, 0, ...toInsert);
  }

  // recast double parfois le `;` d'une directive ("use client";;) quand le body
  // du programme est muté juste après. Nettoyage ciblé (scopé au prologue, ne
  // touche pas un `for(;;)`). cf. 10-case-study-rpbey.md.
  const out = root.toSource({ quote: "single" });
  return out.replace(/^(\s*(['"])use (?:client|strict)\2);;/gm, "$1;");
}

/**
 * Longueur du prologue de directives en tête de programme ("use client",
 * "use strict"…). Les imports injectés doivent venir APRÈS, sinon la directive
 * perd sa position de tête (elle cesse d'être une directive) et recast émet un
 * `;;` parasite. Une directive = ExpressionStatement dont l'expression est un
 * littéral chaîne, en tête de body (avant tout autre type de statement).
 */
function directivePrologueLength(body: any[]): number {
  let i = 0;
  while (i < body.length) {
    const stmt = body[i];
    const isDirective =
      stmt?.type === "ExpressionStatement" &&
      (stmt.expression?.type === "StringLiteral" ||
        (stmt.expression?.type === "Literal" && typeof stmt.expression.value === "string"));
    if (!isDirective) break;
    i++;
  }
  return i;
}

/** Détecte si l'élément contient un <md-icon> ajouté (pour l'import d'effet de bord). */
function needMdIconUsed(el: JSXElement): boolean {
  for (const c of el.children || []) {
    if (
      c.type === "JSXElement" &&
      c.openingElement.name.type === "JSXIdentifier" &&
      c.openingElement.name.name === "md-icon"
    ) {
      return true;
    }
  }
  return false;
}
