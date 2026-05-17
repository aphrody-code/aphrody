<!-- SPDX-License-Identifier: Apache-2.0 -->

# ADR-NNNN: Short Title of the Decision

## Status

One of: Proposed | Accepted | Deprecated | Superseded by ADR-NNNN

Date: YYYY-MM-DD

Author: aphrody-code

## Context

What is the problem we are trying to solve? What forces are at play (technical,
political, social, project-local)? Describe the constraints, the existing
landscape, and any pressure that demands a decision now rather than later.

Keep this section factual. State the moving parts:

- Current state of the codebase (crates affected, dependencies, target triples).
- External constraints (CI budget, licence requirements, supply-chain audits).
- Stakeholder concerns (downstream consumers, peer Claude in `C:\winclean`,
  CI pipelines, packaging targets).

## Decision

State the decision in one paragraph, present tense, active voice: "We will use
X for Y because Z." This is the heart of the ADR — everything above motivates
it and everything below explains its cost.

If the decision has multiple sub-clauses, enumerate them:

1. First sub-decision (e.g. choice of crate version, feature flag).
2. Second sub-decision (e.g. directory layout, cfg-gating strategy).
3. Third sub-decision (e.g. CI gate matrix entry).

## Alternatives Considered

For each rejected alternative, state what it was and why we did not pick it.
This is the section future readers will read first when they ask "why didn't
you just do X?":

- **Alternative A**: brief description. Rejected because [load-bearing reason].
- **Alternative B**: brief description. Rejected because [load-bearing reason].
- **Alternative C**: brief description. Rejected because [load-bearing reason].

Do not strawman. If an alternative had real merit, say so before stating the
disqualifier.

## Consequences

What becomes easier or harder as a result of this decision? Split into:

- **Positive**: capabilities unlocked, risks mitigated, complexity removed.
- **Negative**: trade-offs accepted, new failure modes, future migration cost.
- **Neutral**: changes that are net-zero but worth documenting (e.g. moves a
  problem from one layer to another).

Mention any follow-up ADRs implied by this decision.

## References

- Related ADRs: ADR-NNNN, ADR-MMMM.
- External docs: links to RFCs, vendor docs, crates.io entries, peer-repo paths
  (e.g. `C:\winclean\ai.json`).
- Commits that materialise the decision: `git log` SHAs.
- Issues / PRs that drove the discussion.
