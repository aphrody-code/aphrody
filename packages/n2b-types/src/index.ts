// @generated — DO NOT EDIT MANUALLY.
// Regenerate with: bun run scripts/generate-schema-types.ts
// Source of truth: schema/v2.json

/**
 * This interface was referenced by `N2BReport`'s JSON-Schema
 * via the `definition` "Mode".
 */
export type Mode = "check" | "fix" | "aggressive";
/**
 * This interface was referenced by `N2BReport`'s JSON-Schema
 * via the `definition` "Severity".
 */
export type Severity = "error" | "warn" | "info";

/**
 * Payload schema for n2b scan results (JSON reports). Mirrors the JSON produced by `n2b --report=json`. JSONL mode wraps each object with a `type` discriminator ("meta" for the header, "finding" for subsequent lines).
 */
export interface N2BReport {
  /**
   * URL to this schema.
   */
  $schema?: string;
  /**
   * Schema version, bumped on breaking changes.
   */
  schema_version: 2;
  /**
   * Tool name (historically "node2bun").
   */
  tool: string;
  /**
   * n2b binary semver.
   */
  version: string;
  mode: Mode;
  /**
   * Absolute path of the scanned root.
   */
  root: string;
  files_scanned: number;
  findings_total: number;
  files: FileFix[];
}
/**
 * This interface was referenced by `N2BReport`'s JSON-Schema
 * via the `definition` "FileFix".
 */
export interface FileFix {
  /**
   * Relative path to the scanned root.
   */
  path: string;
  /**
   * True if the file content differs from its pre-scan state (only in --fix / --aggressive / --migrate modes).
   */
  changed: boolean;
  findings: Finding[];
}
/**
 * This interface was referenced by `N2BReport`'s JSON-Schema
 * via the `definition` "Finding".
 */
export interface Finding {
  /**
   * Rule identifier — slash-separated category/name (e.g. 'api/fs-readFileSync'). Immutable: consumers parse this.
   */
  rule_id: string;
  /**
   * Top-level category derived from rule_id prefix.
   */
  category: string;
  severity: Severity;
  /**
   * Heuristic confidence 0..1.
   */
  confidence: number;
  message: string;
  line: number;
  col: number;
  /**
   * Byte offset into the scanned file of the finding start (UTF-8).
   */
  start_byte: number;
  /**
   * Byte offset into the scanned file of the finding end (UTF-8).
   */
  end_byte: number;
  /**
   * Exact text that matched (from source).
   */
  original: string;
  /**
   * Suggested replacement. Omitted entirely when no replacement is known.
   */
  replacement?: string;
  /**
   * True when the rule can be auto-applied by --fix.
   */
  autofix: boolean;
  /**
   * True when the rule is only applied by --aggressive. Omitted when false/unset.
   */
  aggressive?: boolean;
  /**
   * Stable Bun (or external) docs URL for this rule.
   */
  docs_url: string;
  context: Context;
  compat?: Compat;
}
/**
 * Source context around a finding: up to 3 lines before, the finding's line, up to 3 lines after. Consumed by LLM/IDE integrations.
 *
 * This interface was referenced by `N2BReport`'s JSON-Schema
 * via the `definition` "Context".
 */
export interface Context {
  before: string[];
  line: string;
  after: string[];
}
/**
 * Phase 3+ : statut de compat Bun du module hôte. Présent uniquement sur les findings imports/node-* et api/node-*. Optionnel (rétro-compat schema_version=2).
 */
export interface Compat {
  /**
   * Statut de couverture Bun du module Node host. full = tout passe, partial = bug-free sur les chemins courants, missing = pas d'équivalent natif.
   */
  status: "full" | "partial" | "missing";
  /**
   * Nom du module Node concerné (sans le préfixe node:).
   */
  module: string;
  /**
   * Sous-APIs documentées comme non implémentées par Bun. Vide quand status=full.
   */
  missing_apis?: string[];
  /**
   * Équivalent Bun natif suggéré (ex: 'bun:sqlite', 'Bun.serve').
   */
  equivalent?: string;
  /**
   * Polyfill @bun++/node-* recommandé quand status=missing. Optionnel.
   */
  bunpp?: string;
}
/**
 * Bun↔Node compatibility metadata of the host module — Phase 3+. Optional (rétro-compat). Le champ status pilote la sévérité dérivée.
 *
 * This interface was referenced by `N2BReport`'s JSON-Schema
 * via the `definition` "Compat".
 */
export interface Compat1 {
  /**
   * Statut de couverture Bun du module Node host. full = tout passe, partial = bug-free sur les chemins courants, missing = pas d'équivalent natif.
   */
  status: "full" | "partial" | "missing";
  /**
   * Nom du module Node concerné (sans le préfixe node:).
   */
  module: string;
  /**
   * Sous-APIs documentées comme non implémentées par Bun. Vide quand status=full.
   */
  missing_apis?: string[];
  /**
   * Équivalent Bun natif suggéré (ex: 'bun:sqlite', 'Bun.serve').
   */
  equivalent?: string;
  /**
   * Polyfill @bun++/node-* recommandé quand status=missing. Optionnel.
   */
  bunpp?: string;
}
