---
description: Show Bun agent memory, sessions, and dream status
allowed-tools: Bash
shell: bash
---

Show the Bun agent's current state:

```bash
DATA="${CLAUDE_PLUGIN_DATA}"
MEM="$DATA/memory"
SESS="$DATA/sessions"

echo "=== Bun Agent Status ==="
echo "Data dir: $DATA"
echo ""

echo "--- Memory ---"
if [ -d "$MEM" ]; then
  echo "Files: $(ls "$MEM"/*.md 2>/dev/null | wc -l) topic files"
  if [ -f "$MEM/MEMORY.md" ]; then
    echo "Index (MEMORY.md):"
    cat "$MEM/MEMORY.md"
  else
    echo "(no MEMORY.md index)"
  fi
  echo ""
  if [ -f "$MEM/.consolidate-lock" ]; then
    echo "Last dream: $(stat -c '%y' "$MEM/.consolidate-lock" 2>/dev/null || stat -f '%Sm' "$MEM/.consolidate-lock" 2>/dev/null)"
  else
    echo "Last dream: never"
  fi
else
  echo "(no memory directory — run /bun-agent:dream to initialize)"
fi

echo ""
echo "--- Sessions ---"
if [ -d "$SESS" ]; then
  COUNT=$(ls "$SESS"/*.jsonl 2>/dev/null | wc -l)
  echo "Total: $COUNT session files"
  echo "Recent:"
  ls -lt "$SESS"/*.jsonl 2>/dev/null | head -5
else
  echo "(no sessions recorded yet)"
fi
```

Execute this and present the results.
