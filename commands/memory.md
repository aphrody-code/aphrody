---
description: Read or search the Bun agent's persistent memory files
allowed-tools: Bash
argument-hint: "[search term or topic file name]"
shell: bash
---

Read or search the agent's memory:

```bash
DATA="${CLAUDE_PLUGIN_DATA}"
MEM="$DATA/memory"
QUERY="$ARGUMENTS"

if [ ! -d "$MEM" ]; then
  echo "No memory directory yet. Run /bun-agent:dream to initialize."
  exit 0
fi

if [ -z "$QUERY" ]; then
  echo "=== All Memory Files ==="
  for f in "$MEM"/*.md; do
    [ -f "$f" ] || continue
    NAME=$(basename "$f")
    DESC=$(head -10 "$f" | grep "^description:" | sed 's/description: *//')
    TYPE=$(head -10 "$f" | grep "^type:" | sed 's/type: *//')
    echo "  [$TYPE] $NAME — $DESC"
  done
else
  echo "=== Searching memories for: $QUERY ==="
  grep -rin "$QUERY" "$MEM"/*.md 2>/dev/null || echo "No matches found."
fi
```

Execute and present the results. If the user asked about a specific topic, also read the relevant memory file in full.
