# a2ui — Agent-to-UI protocol reference (excluded)

This directory is **intentionally gitignored** (see `.gitignore` §21).

The full a2ui source (~119 MB) is Google's reference implementation of the Agent-to-UI streaming protocol. We don't vendor it.

## Re-clone (if needed)

```bash
gh repo clone google/A2UI packages/a2ui
```

## Production consumption

aphrody consumes the A2UI protocol via:
- `crates/a2a*` workspace members (Rust impl)
- `@a2ui/*` npm packages when needed for TS projects

Upstream : <https://github.com/google/A2UI>
