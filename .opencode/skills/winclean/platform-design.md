# Platform Design Skill

> Designing platform APIs and CLI tools with best practices.

## CLI Design Principles

### 1. Consistent Interface
```powershell
# Good: Consistent verb-noun structure
winclean scan
winclean profile
winclean debloat apply

# Avoid: Inconsistent commands
winclean scan
winclean get-profile
winclean apply-debloat
```

### 2. Proper Exit Codes
```powershell
# 0 = success
# 1 = general error
# 2 = misuse (bad arguments)
# 3 = not implemented
exit 0  # Success
exit 1  # Error
```

### 3. Progress & Feedback
```powershell
Write-Host "Scanning system..." -ForegroundColor Cyan
Write-Host "✓ Found 42 bloatware packages" -ForegroundColor Green

# Use Write-Progress for long operations
Write-Progress -Activity "Scanning" -PercentComplete 50
```

## Output Formats

### JSON for Machine Consumption
```powershell
$result = @{
    os = "Windows 11 24H2"
    build = 26100
    bloatware = @(...)
} | ConvertTo-Json -Depth 10

$result | Out-File "scan.json" -Encoding UTF8
```

### Formatted Text for Humans
```powershell
Write-Host @"
=== System Scan ===
OS: Windows 11 24H2 (Build 26100)
Bloatware: 42 packages
Telemetry: 8 active tasks
"@
```

## Configuration

- Global config: `$env:APPDATA\winclean\config.json`
- Project config: `.winclean.json` in project root
- Secrets: Never store in config, use env vars

## Testing

```powershell
# Unit tests with Pester
Invoke-Pester tests/

# Integration tests
pwsh -File tests/integration.ps1
```
