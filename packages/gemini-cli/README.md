# gemini-cli — Google Gemini CLI reference (excluded)

This directory is **intentionally gitignored** (see `.gitignore` §21).

The full gemini-cli monorepo source (~103 MB, ~28k files) is Google's reference Gemini CLI. We don't vendor it.

## Re-clone (if needed)

```bash
gh repo clone google-gemini/gemini-cli packages/gemini-cli
```

## Production consumption

aphrody integrates Gemini CLI via:
- Sub-command `aphrody gemini ...` which delegates to the installed `gemini` binary on PATH
- Or to the Bun-bundled `packages/gemini-cli/bundle/gemini.js` if cloned locally

Upstream : <https://github.com/google-gemini/gemini-cli>

## Install upstream globally

```bash
bun install -g @google/gemini-cli
gemini --help
```

aphrody auto-detects `gemini` on PATH and delegates the OAuth flow / chat invocations to it.
