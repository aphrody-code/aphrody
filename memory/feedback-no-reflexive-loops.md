---
name: feedback-no-reflexive-loops
description: "Don't mechanically repeat tool calls (esp. bxc browser / workflows) — think whether the action is even needed before acting"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

The user said "infinite loop, pause et réfléchi" **twice** (2026-06-04) when I reflexively kept driving the bxc browser / spawning workflows instead of reasoning first.

**Why:** A *future* tournament (Stardust #2 / T_SS2, dated 7 June) has no bracket → there was **nothing to scrape**, yet I kept navigating bxc to its `/module` to "check the phase". The correct path was trivial: parse the announcement → upsert the row → done (announce phase). I was pattern-matching the previous (completed) tournament instead of thinking about THIS one.

**How to apply:** Before any tool call — especially bxc browser navigations, repeated probes, or launching a Workflow — ask "is this action actually necessary to reach the goal, given THIS input's state?" Prefer the simplest direct path. Never repeat an approach that was just rejected/failed. When stuck or looping, stop and reason in text first. Complements [[feedback-no-questions-direct-action]] (be direct) — but direct ≠ mechanical: be direct AND deliberate.
