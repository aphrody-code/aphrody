<!-- SPDX-License-Identifier: Apache-2.0 -->

# Reverse-Engineering Audit: Gemini CLI Skills Subsystem

Date: 2026-05-17
Scope: Full schema + loader + runtime invocation pattern for gemini-cli SKILL.md format
Objective: Enable aphrody to consume gemini-cli skills without semantic drift.

## 1. Inventory: 11 Built-In Skills

| Skill Name | LOC | Frontmatter Keys | Sibling Assets |
|---|---|---|---|
| async-pr-review | 44 | name, description | policy.toml, 2x shell scripts |
| behavioral-evals | 56 | name, description | References to nested docs |
| ci | 66 | name, description | None |
| code-reviewer | 65 | name, description | None |
| docs-changelog | 168 | name, description | None |
| docs-writer | 194 | name, description | None |
| github-issue-creator | 76 | name, description | None |
| pr-address-comments | 13 | name, description | References to fetch-pr-info.js |
| pr-creator | 93 | name, description | None |
| review-duplication | 69 | name, description | None |
| string-reviewer | 98 | name, description | References to word-list.md |

Total: 942 LOC across 11 skills. No triggers, when_to_use, or od.{mode,category} fields present.

## 2. Canonical Schema Reference

Required Frontmatter Keys:
- name (string): Skill identifier; sanitized via regex /[:\/<>*?"|]/g -> -
- description (string): Single-line or multi-line YAML block scalar (| or >)

Optional Frontmatter Keys:
- NONE. Gemini CLI explicitly does NOT support:
  - triggers (aphrody + open-design use this)
  - when_to_use (Claude Code uses this)
  - od.{mode, category, ...} (open-design uses this)
  - metadata (vercel-labs uses this)
  - license (vercel-labs uses this)

Key Invariant: Frontmatter parsed ONLY for name + description. All other content is body.

## 3. Loader Walkthrough

Entrypoint: C:/worktree/gemini-cli/packages/core/src/skills/skillLoader.ts:115-159

Discovery: loadSkillsFromDir(dir: string) uses glob patterns ['SKILL.md', '*/SKILL.md']
with cwd set to absolute search path, absolute: true, ignoring node_modules and .git.

File Parsing: loadSkillFromFile(filePath: string) uses FRONTMATTER_REGEX 
^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n([\s\S]*))?

Fallback parser chain: (1) Try js-yaml load() on frontmatter. (2) Fall back to line-by-line parser 
if YAML fails (handles colons in descriptions). Sanitization: 
frontmatter.name.replace(/[:\/<>*?"|]/g, '-')

## 4. Runtime Invocation Flow

User provides task -> System prompt lists available skills -> Model calls activate_skill(name) 
-> ActivateSkillTool.execute() -> skillManager.getSkill(name) returns SkillDefinition 
-> skill.body wrapped in <activated_skill> tags -> Returned to model as LLM content 
-> Model continues reasoning with embedded instructions.

Key Implementation: C:/worktree/gemini-cli/packages/core/src/tools/activate-skill.ts:111-154

The activate_skill tool injects skill.body into XML tags and returns folder structure of sibling files.

System prompt guidance (snippets.ts): "Once a skill is activated via activate_skill, 
its instructions and resources are returned wrapped in <activated_skill> tags. 
You MUST treat the content within <instructions> as expert procedural guidance, 
prioritizing these specialized rules and workflows over your general defaults for 
the duration of the task."

## 5. Schema Comparison Table

| Aspect | Gemini | Open-Design | Claude Code | Vercel |
|---|---|---|---|---|
| name | Required | Required | Required | Required |
| description | Required | Required | Required | Required |
| triggers | No | Yes (List[str]) | No | No |
| when_to_use | No | No | Yes (str) | No |
| od.{...} | No | Yes (nested) | No | No |
| metadata | No | No | No | Yes (Map) |
| license | No | No | No | Yes |
| Invocation | Tool-based | Narrative | Narrative | Narrative |
| Sibling Assets | Some | Few | Few | Rare |

Key Finding: Gemini CLI is ONLY schema with: (1) No trigger phrases. (2) Tool-based skill invocation. 
(3) policy.toml bundling.

## 6. Migration Matrix

7/11 (63%) FULLY COMPATIBLE: ci, code-reviewer, docs-changelog, docs-writer, 
github-issue-creator, pr-creator, review-duplication. These have self-contained body text.

4/11 (36%) PARTIAL: async-pr-review (needs sibling script paths), 
behavioral-evals (file: URLs), pr-address-comments (fetch-pr-info.js reference), 
string-reviewer (word-list.md reference).

Recommended Shim: In aphrody loader when schema == "gemini": 
(1) Load frontmatter (name, description) losslessly. 
(2) Resolve relative paths in body relative to dirname(skillFile). 
(3) Do NOT emit triggers. 
(4) Prepend note about available sibling resources.

## 7. Open Questions

Q1: Trigger Phrase Semantics
Issue: Zero trigger phrases in SKILL.md. How does user invoke without triggers?
Hypothesis: User calls activate_skill tool with skill name. No fuzzy matching.
Impact: Aphrody fuzzy trigger matching does NOT apply. Must infer or use explicit tool call.

Q2: Policy Enforcement Scope
Issue: async-pr-review/policy.toml defines 100+ allowed commands. 
Per-skill or global enforcement?
Impact: Unknown if policy.toml auto-mounts with skill activation.
Recommendation: Clarify with upstream.

Q3: Nested Procedure Files
Issue: Skills reference external .md files (behavioral-evals->creating.md, 
string-reviewer->word-list.md). Auto-discovered?
Impact: Loader must extend to glob **/*.md alongside SKILL.md.
Recommendation: Update skillLoader to discover and expose sibling .md files.

## Conclusion

Gemini CLI SKILL.md is schema-compatible but runtime-incompatible with aphrody.

Frontmatter: Compatible (name + description only).
Triggers: Incompatible (none defined; aphrody fuzzy matching inapplicable).
Invocation: Incompatible (tool-based vs aphrody narrative injection).
Assets: Partially incompatible (some sibling files; loader extension needed).

VERDICT: PARTIAL (63% fully compatible, 36% require minimal shim).
EFFORT: 2-3 days (loader extension, sibling path validation, test coverage).

