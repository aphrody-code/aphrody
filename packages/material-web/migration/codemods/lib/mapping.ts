/**
 * lib/mapping.ts — Table de mapping partagée MUI -> @aphrody/m3-react.
 *
 * SOURCE DE VÉRITÉ : `migration/00-CONVENTIONS.md` §3 (mapping canonique) et §4
 * (règles props/events). Toutes les valeurs ici proviennent du contrat ; aucun
 * élément `md-*` ni wrapper inventé (cf. §7.2). La liste des 120 éléments réels
 * est dans `migration/scripts/md-elements.txt` (copie de
 * `packages/react/md-elements.txt`). Mapping confronté à un corpus applicatif
 * anonymisé.
 *
 * Ce module est consommé par TOUS les transforms jscodeshift de `transforms/`.
 */

/** Nom du package des wrappers React (cf. §2). */
export const M3_PKG = "@aphrody/m3-react";

/** Package(s) MUI source à intercepter (cf. §0). */
export const MUI_PKGS = ["@mui/material"];

/**
 * Composants dont le wrapper dépend de la prop `variant` (cf. §3).
 * `default` = wrapper choisi quand `variant` est absent (défaut MUI documenté).
 */
export interface VariantMap {
  /** prop MUI qui pilote le choix du wrapper (souvent "variant"). */
  prop: string;
  /** valeur de prop -> wrapper React. */
  byValue: Record<string, string>;
  /** wrapper par défaut si la prop est absente / non reconnue. */
  default: string;
}

/**
 * Composants MUI variant-dépendants (cf. §3).
 * NB : pour MUI, `Button` défaut = `variant="text"` côté MUI, MAIS le contrat
 * §3 fixe explicitement "variant=contained (défaut)" -> md-filled-button comme
 * choix de migration canonique. On suit le contrat.
 */
export const VARIANT_COMPONENTS: Record<string, VariantMap> = {
  Button: {
    prop: "variant",
    byValue: {
      contained: "MdFilledButton",
      outlined: "MdOutlinedButton",
      text: "MdTextButton",
      // valeurs M3 supplémentaires acceptées si présentes :
      elevated: "MdElevatedButton",
      tonal: "MdFilledTonalButton",
    },
    // §3 : "variant=contained (défaut)" -> md-filled-button
    default: "MdFilledButton",
  },
  TextField: {
    prop: "variant",
    byValue: {
      filled: "MdFilledTextField",
      outlined: "MdOutlinedTextField",
      // MUI "standard" n'a pas d'équivalent M3 -> fallback filled (TODO marqueur)
      standard: "MdFilledTextField",
    },
    // §3 : "variant=filled (défaut M3)" -> md-filled-text-field
    default: "MdFilledTextField",
  },
  Select: {
    prop: "variant",
    byValue: {
      filled: "MdFilledSelect",
      outlined: "MdOutlinedSelect",
      standard: "MdFilledSelect",
    },
    default: "MdFilledSelect",
  },
  NativeSelect: {
    prop: "variant",
    byValue: {
      filled: "MdFilledSelect",
      outlined: "MdOutlinedSelect",
      standard: "MdFilledSelect",
    },
    default: "MdFilledSelect",
  },
};

/**
 * Mapping 1:1 simple (composant MUI -> wrapper unique), cf. §3.
 * Les sous-composants (DialogTitle, CardHeader, MenuItem, ...) sont gérés
 * séparément dans SLOTTED_CHILDREN / RENAME_CHILDREN.
 */
export const SIMPLE_COMPONENTS: Record<string, string> = {
  Checkbox: "MdCheckbox",
  Radio: "MdRadio",
  Switch: "MdSwitch",
  Slider: "MdSlider",
  Divider: "MdDivider",
  Dialog: "MdDialog",
  Menu: "MdMenu",
  MenuList: "MdMenu",
  MenuItem: "MdMenuItem",
  List: "MdList",
  ListItem: "MdListItem",
  LinearProgress: "MdLinearProgress",
  CircularProgress: "MdCircularProgress",
  Tabs: "MdTabs",
  Tab: "MdPrimaryTab",
  Tooltip: "MdTooltip",
  Snackbar: "MdSnackbar",
  Badge: "MdBadge",
  Icon: "MdIcon",
  Fab: "MdFab",
  // IconButton : variant-dépendant via `color` mais wrapper de base = MdIconButton (§3)
  IconButton: "MdIconButton",
  // Card de base : md-card (les variantes elevated/filled/outlined existent aussi)
  Card: "MdCard",
  CardActions: "MdCard", // contenu slotté -> géré en TODO
  // --- ex-GAP désormais livrés (vérifié 2026-05-29, packages/react/wrappers) ---
  Avatar: "MdAvatar",
  AvatarGroup: "MdAvatarGroup",
  Alert: "MdAlert",
  Breadcrumbs: "MdBreadcrumbs",
  Rating: "MdRating",
  Skeleton: "MdSkeleton",
  Backdrop: "MdBackdrop",
  Popover: "MdPopover",
  MobileStepper: "MdMobileStepper",
  Link: "MdLink",
  // Paper M3 = surface (md-surface). Layout pur -> préférer <div> + Tailwind si
  // le Paper ne sert qu'à grouper sans élévation/teinte de surface.
  Paper: "MdSurface",
  // --- composants confirmés par le corpus applicatif de référence ---
  Pagination: "MdPaginator",
  Autocomplete: "MdAutocomplete",
  Accordion: "MdAccordion",
  AppBar: "MdTopAppBar",
  Toolbar: "MdToolbar",
  BottomNavigation: "MdNavigationBar",
  // ToggleButtonGroup MUI = segmented button set M3 ; ToggleButton = segment
  ToggleButtonGroup: "MdOutlinedSegmentedButtonSet",
  ToggleButton: "MdOutlinedSegmentedButton",
  // Drawer : par défaut le tiroir de navigation standard (variante modal possible)
  Drawer: "MdNavigationDrawer",
  // Chip : M3 distingue les genres (assist/filter/input/suggestion). Défaut assist
  // -> TODO de re-typage selon l'intention (onDelete -> input, onClick -> assist…).
  Chip: "MdAssistChip",
};

/**
 * Composants MUI SANS équivalent md (gaps, cf. §3 + 05-gap-analysis.md).
 * Le codemod NE les transforme PAS : il insère un marqueur MIGRATION-TODO.
 */
export const GAP_COMPONENTS: Record<string, string> = {
  // Gaps RÉELS restants (vérifié 2026-05-29). Avatar/AvatarGroup/Alert/Skeleton/
  // Breadcrumbs/Rating/Backdrop/Popover/MobileStepper/Link sont désormais livrés
  // -> voir SIMPLE_COMPONENTS. Ne restent ici que primitives et transitions sans
  // élément md dédié (résolues par tokens / Tailwind / Popover API natif).
  Modal: "primitive (gap) -> md-dialog (modal) + md-backdrop pour le scrim",
  Popper: "primitive (gap) -> Popover API native + md-menu / md-popover",
  Collapse: "transition (gap) -> @aphrody/m3-motion + motion tokens",
  Fade: "transition (gap) -> @aphrody/m3-motion + motion tokens",
  Grow: "transition (gap) -> @aphrody/m3-motion + motion tokens",
  Slide: "transition (gap) -> @aphrody/m3-motion + motion tokens",
  Zoom: "transition (gap) -> @aphrody/m3-motion + motion tokens",
  CssBaseline: "pas d-element md -> reset CSS + import des tokens M3",
  ScopedCssBaseline: "pas d-element md -> reset CSS scopé + tokens M3",
};

/**
 * Composants de LAYOUT MUI -> <div> + Tailwind (cf. §3 "Layout" + §6).
 * Le codemod les transforme en <div> et insère un TODO sur la conversion des
 * props de layout (`spacing`, `direction`, `container`, `item`...).
 */
export const LAYOUT_COMPONENTS: Record<string, string> = {
  Box: "div",
  Container: "div",
  Stack: "div",
  Grid: "div",
  Grid2: "div",
  // NB : Paper n'est PLUS ici. M3 -> md-surface (SIMPLE_COMPONENTS). Si un Paper
  // ne sert que de conteneur de layout sans teinte/élévation, le repasser <div>
  // manuellement reste préférable.
  Typography: "MdType", // §3 : md-type OU classes typescale. On choisit le wrapper.
};

/**
 * Renommage direct des sous-composants enfants (élément JSX uniquement),
 * sans logique de slot complexe. Ex : <MenuItem> -> <MdMenuItem>.
 */
export const RENAME_CHILDREN: Record<string, string> = {
  MenuItem: "MdMenuItem",
  ListItem: "MdListItem",
  Tab: "MdPrimaryTab",
};

/**
 * Sous-composants MUI qui deviennent du CONTENU SLOTTÉ d'un parent md
 * (cf. §4 "Slots"). Le codemod ne peut pas toujours restructurer en toute
 * sécurité -> il renomme + ajoute l'attribut `slot` quand c'est trivial, sinon
 * marqueur MIGRATION-TODO.
 *
 * map: composant MUI -> { parent, slot, replaceWith }
 */
export interface SlotRule {
  /** parent md attendu (pour la doc/TODO). */
  parent: string;
  /** valeur de l'attribut slot à poser sur le nouvel élément. */
  slot: string;
  /** élément de remplacement (souvent un <div> slotté). */
  replaceWith: string;
}

export const SLOTTED_CHILDREN: Record<string, SlotRule> = {
  // Dialog (§3) : slots headline / content / actions
  DialogTitle: { parent: "MdDialog", slot: "headline", replaceWith: "div" },
  DialogContent: { parent: "MdDialog", slot: "content", replaceWith: "div" },
  DialogContentText: {
    parent: "MdDialog",
    slot: "content",
    replaceWith: "div",
  },
  DialogActions: { parent: "MdDialog", slot: "actions", replaceWith: "div" },
  // Card (§3) : sous-composants slottés
  CardContent: { parent: "MdCard", slot: "content", replaceWith: "div" },
  CardHeader: { parent: "MdCard", slot: "headline", replaceWith: "div" },
  CardMedia: { parent: "MdCard", slot: "media", replaceWith: "div" },
  // Alert : AlertTitle -> titre slotté (confirmé par le corpus de référence)
  AlertTitle: { parent: "MdAlert", slot: "title", replaceWith: "div" },
  // Accordion : header/contenu slottés (md-accordion)
  AccordionSummary: {
    parent: "MdAccordion",
    slot: "summary",
    replaceWith: "div",
  },
  AccordionDetails: {
    parent: "MdAccordion",
    slot: "content",
    replaceWith: "div",
  },
  // BottomNavigation : action -> onglet de la navigation bar
  BottomNavigationAction: {
    parent: "MdNavigationBar",
    slot: "destination",
    replaceWith: "MdNavigationTab",
  },
};

/**
 * Mapping des PROPS communes MUI -> md (cf. §4).
 * - `rename` : changement de nom d'attribut.
 * - `drop`   : attribut sans équivalent -> retiré + TODO.
 * - `iconSlot` : startIcon/endIcon -> <md-icon slot="..."> (traitement spécial).
 */
export const PROP_RENAME: Record<string, string> = {
  // valeur/label restent souvent identiques ; on documente les exceptions
  // (à étendre composant par composant dans 01-component-mapping.md).
};

/** Props MUI à retirer avec un marqueur TODO (pas d'équivalent direct md). */
export const PROP_DROP_WITH_TODO = new Set<string>([
  "sx",
  "color", // palette M2 -> tokens M3, géré par thème (TODO)
  "size", // md gère via tokens/density, pas de prop universelle
  "fullWidth", // -> width via Tailwind/style host
  "disableElevation",
  "disableRipple",
]);

/** Tous les wrappers connus, pour générer les imports nommés. */
export function allWrappers(): Set<string> {
  const s = new Set<string>();
  for (const v of Object.values(VARIANT_COMPONENTS)) {
    s.add(v.default);
    for (const w of Object.values(v.byValue)) s.add(w);
  }
  for (const w of Object.values(SIMPLE_COMPONENTS)) s.add(w);
  for (const w of Object.values(RENAME_CHILDREN)) s.add(w);
  for (const w of Object.values(LAYOUT_COMPONENTS)) if (w.startsWith("Md")) s.add(w);
  // les sous-composants slottés remplacés par un vrai élément md (pas un <div>)
  for (const r of Object.values(SLOTTED_CHILDREN))
    if (r.replaceWith.startsWith("Md")) s.add(r.replaceWith);
  return s;
}
