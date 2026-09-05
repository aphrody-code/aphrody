/**
 * lib/jsx-helpers.ts — utilitaires AST partagés (jscodeshift / ast-types).
 *
 * Conforme à 00-CONVENTIONS.md §4 (props/events/slots). Aucune dépendance
 * runtime hors jscodeshift. Tous les helpers sont purs et testables.
 */
import type { API, ASTPath, JSXAttribute, JSXElement, JSXOpeningElement } from "jscodeshift";

/** Récupère le nom textuel d'un élément JSX ouvrant (ex: "Button"). */
export function jsxName(open: JSXOpeningElement): string | null {
  const n = open.name;
  if (n.type === "JSXIdentifier") return n.name;
  // Membre (ex: <Foo.Bar>) : on ne gère pas -> null
  return null;
}

/** Lit la valeur littérale string d'un attribut JSX (variant="outlined"). */
export function attrStringValue(attr: JSXAttribute): string | null {
  const v = attr.value;
  if (!v) return null; // attribut booléen (ex: disabled)
  if (v.type === "StringLiteral" || (v.type as string) === "Literal") {
    // @ts-expect-error variantes de parser (babel vs ts)
    return typeof v.value === "string" ? v.value : null;
  }
  if (v.type === "JSXExpressionContainer") {
    const e = v.expression;
    if (e.type === "StringLiteral" || (e.type as string) === "Literal") {
      // @ts-expect-error idem
      return typeof e.value === "string" ? e.value : null;
    }
  }
  return null;
}

/** Trouve un attribut JSX par nom sur un élément ouvrant. */
export function findAttr(open: JSXOpeningElement, name: string): JSXAttribute | null {
  for (const a of open.attributes || []) {
    if (a.type === "JSXAttribute" && a.name.type === "JSXIdentifier" && a.name.name === name) {
      return a;
    }
  }
  return null;
}

/** Retire un attribut JSX par nom. Retourne true si retiré. */
export function removeAttr(open: JSXOpeningElement, name: string): boolean {
  const before = (open.attributes || []).length;
  open.attributes = (open.attributes || []).filter(
    (a) => !(a.type === "JSXAttribute" && a.name.type === "JSXIdentifier" && a.name.name === name),
  );
  return (open.attributes?.length || 0) < before;
}

/** Renomme l'élément JSX (ouvrant + fermant). */
export function renameElement(el: JSXElement, newName: string): void {
  if (el.openingElement.name.type === "JSXIdentifier") {
    el.openingElement.name.name = newName;
  }
  if (el.closingElement && el.closingElement.name.type === "JSXIdentifier") {
    el.closingElement.name.name = newName;
  }
}

/**
 * Insère un marqueur MIGRATION-TODO pour le non-automatisable.
 *
 * IMPORTANT : un commentaire `/* *\/` brut placé entre des enfants JSX est
 * INVALIDE (il faut `{/* *\/}`). On gère donc deux cas :
 *  - élément JSX enfant d'un autre élément JSX -> on insère un sibling
 *    `{/* MIGRATION-TODO ... *\/}` (JSXExpressionContainer + commentaire interne)
 *    juste avant l'élément, ce qui compile.
 *  - sinon (statement, attribut...) -> leading comment classique sur le noeud.
 *
 * Idempotent : ne ré-insère pas un marqueur identique déjà présent juste avant.
 */
export function addMigrationTodo(j: API["jscodeshift"], path: ASTPath<any>, message: string): void {
  const text = ` MIGRATION-TODO: ${message} `;
  const parent = path.parent;
  const inJsxChildren =
    parent &&
    parent.node &&
    (parent.node.type === "JSXElement" || parent.node.type === "JSXFragment");

  if (inJsxChildren && typeof path.name === "number") {
    // On NE splice PAS pendant la traversée (les index des siblings
    // décaleraient et les commentaires se retrouveraient mal placés). On
    // empile le message sur le noeud ; `flushMigrationTodos` insère les
    // conteneurs après la traversée, en ordre d'index décroissant.
    const node: any = path.node;
    node.__migrationTodos = node.__migrationTodos || [];
    if (!node.__migrationTodos.includes(text)) node.__migrationTodos.push(text);
    return;
  }

  // fallback : leading comment classique (statement, attribut...)
  const node = path.node;
  node.comments = node.comments || [];
  if (!node.comments.some((c: any) => c.value === text)) {
    node.comments.push({
      type: "CommentBlock",
      value: text,
      leading: true,
      trailing: false,
    } as any);
  }
}

/**
 * Construit un `{/* texte *\/}` (JSXExpressionContainer) imprimable par recast.
 */
function makeCommentContainer(j: API["jscodeshift"], text: string): any {
  const empty: any = j.jsxEmptyExpression();
  // recast imprime le commentaire d'un JSXEmptyExpression sur une seule ligne
  // `{/* ... *\/}` quand il est marqué `trailing` (et non `leading`, qui place
  // le `}` de fermeture sur une nouvelle ligne avec recast/jscodeshift 17+).
  // Donne `{/* ... *\/}` valide ET compact en JSX.
  empty.comments = [
    {
      type: "CommentBlock",
      value: text,
      leading: false,
      trailing: true,
    } as any,
  ];
  return j.jsxExpressionContainer(empty);
}

/**
 * Insère réellement les marqueurs empilés par `addMigrationTodo` (cas enfant
 * JSX) sous forme de `{/* MIGRATION-TODO ... *\/}` AVANT chaque élément cible.
 * À appeler une fois après toute la traversée. Insère en ordre d'index
 * décroissant par parent pour ne pas décaler les positions suivantes.
 */
export function flushMigrationTodos(j: API["jscodeshift"], root: any): void {
  // 1) Collecte (sans muter) : pour chaque parent, la liste des positions à
  //    insérer. La position est calculée par identité du noeud enfant (et non
  //    par un index figé qui se décalerait après un premier splice).
  const byParent = new Map<any, { node: any; texts: string[] }[]>();
  root.find(j.JSXElement).forEach((path: ASTPath<any>) => {
    const node: any = path.node;
    const todos: string[] | undefined = node.__migrationTodos;
    if (!todos || todos.length === 0) return;
    const parent = path.parent;
    if (
      !parent ||
      !parent.node ||
      (parent.node.type !== "JSXElement" && parent.node.type !== "JSXFragment")
    ) {
      delete node.__migrationTodos;
      return;
    }
    const list = byParent.get(parent.node) || [];
    list.push({ node, texts: [...todos] });
    byParent.set(parent.node, list);
    delete node.__migrationTodos;
  });

  // 2) Application par parent, en ordre d'index DÉCROISSANT : on retrouve la
  //    position courante de l'enfant par identité au moment de l'insertion,
  //    ce qui reste correct même après les splices précédents.
  for (const [parentNode, entries] of byParent) {
    const children: any[] = parentNode.children;
    // trier par index courant décroissant
    entries.sort((a, b) => children.indexOf(b.node) - children.indexOf(a.node));
    for (const { node, texts } of entries) {
      const idx = children.indexOf(node);
      if (idx < 0) continue;
      const containers = texts.map((t) => makeCommentContainer(j, t));
      children.splice(idx, 0, ...containers);
    }
  }
}

/**
 * Réécrit un handler onChange MUI `(e, value) => ...` ou `(e) => ...` pour
 * material-web où la valeur se lit sur `e.target.value` (cf. §4 controlled
 * components). On ne réécrit PAS le corps automatiquement (risqué) : on pose
 * un TODO si le handler utilise un 2e paramètre `value`.
 */
export function onChangeUsesSecondArg(attr: JSXAttribute): boolean {
  const v = attr.value;
  if (!v || v.type !== "JSXExpressionContainer") return false;
  const e = v.expression;
  if (e.type === "ArrowFunctionExpression" || e.type === "FunctionExpression") {
    return (e.params?.length || 0) >= 2;
  }
  return false;
}

/** Construit un JSXAttribute `slot="..."` via le builder jscodeshift. */
export function makeSlotAttr(j: API["jscodeshift"], slot: string): JSXAttribute {
  return j.jsxAttribute(j.jsxIdentifier("slot"), j.stringLiteral(slot));
}
