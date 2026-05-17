---
description: Remove specific entries from Bun agent memory
allowed-tools: Bash, Read
argument-hint: "[topic file name or search term]"
shell: bash
---

Remove memory entries matching the argument. First show what will be affected, then delete.

```bash
DATA="${CLAUDE_PLUGIN_DATA}"
MEM="$DATA/memory"
TARGET="$ARGUMENTS"

if [ ! -d "$MEM" ]; then
  echo "No memory directory."
  exit 0
fi

if [ -z "$TARGET" ]; then
  echo "Usage: /bun-agent:forget <filename or search term>"
  echo "Available files:"
  ls "$MEM"/*.md 2>/dev/null | while read f; do echo "  $(basename "$f")"; done
  exit 0
fi

# Check if it's a direct filename
if [ -f "$MEM/$TARGET" ]; then
  echo "Found: $MEM/$TARGET"
  echo "--- Content ---"
  cat "$MEM/$TARGET"
  echo ""
  echo "Deleting $TARGET..."
  rm "$MEM/$TARGET"
  # Remove from MEMORY.md index
  if [ -f "$MEM/MEMORY.md" ]; then
    sed -i "/$TARGET/d" "$MEM/MEMORY.md"
  fi
  echo "Done. Memory '$TARGET' removed."
elif [ -f "$MEM/${TARGET}.md" ]; then
  echo "Found: $MEM/${TARGET}.md"
  cat "$MEM/${TARGET}.md"
  echo ""
  rm "$MEM/${TARGET}.md"
  if [ -f "$MEM/MEMORY.md" ]; then
    sed -i "/${TARGET}.md/d" "$MEM/MEMORY.md"
  fi
  echo "Done. Memory '${TARGET}.md' removed."
else
  echo "No exact match. Searching for '$TARGET'..."
  grep -rln "$TARGET" "$MEM"/*.md 2>/dev/null || echo "No matches found."
fi
```

Execute and confirm the result to the user.
