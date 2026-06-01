#!/usr/bin/env bash
# color-guard.sh — PostToolUse nudge: flag hardcoded colors in just-edited UI files
# and remind Claude to use Material Design 3 `--md-sys-color-*` roles instead.
#
# Non-blocking: emits an `additionalContext` nudge via JSON, never fails the edit.
# Portable: parses the hook stdin JSON without requiring jq.
set -euo pipefail

payload="$(cat)"

# Extract the edited file path (flat field in the PostToolUse payload).
file="$(printf '%s' "$payload" \
	| sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
	| head -n1)"

[ -n "$file" ] || exit 0
[ -f "$file" ] || exit 0

# Only inspect UI/styling files.
case "$file" in
	*.css | *.scss | *.tsx | *.jsx | *.ts | *.html | *.svelte | *.vue) ;;
	*) exit 0 ;;
esac

# Skip files whose JOB is to define raw color values (token sources, generated
# palettes, dynamic-color output, theme seeds) — hardcoded hex is legitimate there.
case "$file" in
	*tokens* | *palette* | *dynamic-color* | *theme* | *_config* | *.codepoints | *node_modules*) exit 0 ;;
esac

# A hardcoded color is a hex (#rgb..#rrggbbaa) or rgb()/hsl() literal that is NOT
# already wrapped in a CSS var() and NOT a custom-property *definition* line
# (--md-ref-* / --md-sys-* legitimately hold the raw value).
hits="$(grep -nE '#[0-9a-fA-F]{3,8}\b|\b(rgb|rgba|hsl|hsla)\s*\(' "$file" 2>/dev/null \
	| grep -vE 'var\(|--md-(ref|sys)-|//|/\*' \
	| grep -iE 'color|background|fill|stroke|border|box-shadow|outline|sx=|style=' \
	| head -n5 || true)"

[ -n "$hits" ] || exit 0

count="$(printf '%s\n' "$hits" | grep -c . || true)"
msg="material-design: $count hardcoded color(s) in $(basename "$file"). Material Design 3 uses semantic color ROLES, not raw values — replace with var(--md-sys-color-<role>) (primary, surface, on-surface, outline, error, ...) so the UI follows the theme, dark mode and dynamic color. First lines: $(printf '%s' "$hits" | tr '\n' '|')"

# Emit a non-blocking context nudge (JSON on stdout).
printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":%s}}\n' \
	"$(printf '%s' "$msg" | sed 's/\\/\\\\/g; s/"/\\"/g' | sed 's/^/"/; s/$/"/')"
exit 0
