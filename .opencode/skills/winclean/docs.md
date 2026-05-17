# Documentation Skill

> Creating and maintaining technical documentation.

## Markdown Best Practices

### Headers
```markdown
# Title (only one per file)
## Section
### Subsection
#### Detail
```

### Code Blocks
```markdown
```powershell
# Use language identifier
Get-Process | Where-Object CPU -gt 10
```
```

### Tables
```markdown
| Column 1 | Column 2 |
|----------|----------|
| Value 1  | Value 2  |
```

## WinClean Documentation Structure

```
C:\winclean\docs\
├── README.md          # Main entry point
├── QUICKSTART.md      # Getting started guide
├── ARCHITECTURE.md    # System design
├── API.md             # API reference
├── TROUBLESHOOTING.md # Common issues
└── CHANGELOG.md       # Version history
```

## MCP Documentation Server

When the `docs` MCP server is connected:
- Query documentation directly
- Use semantic search for finding relevant docs

## Style Guide

1. **French** for user-facing content
2. **English** for code, identifiers, commit messages
3. Keep lines under 100 characters
4. Use active voice
5. Include examples

## Tools

- Use oxlint for markdown linting
- Use oxfmt for formatting
