# PE Symbol Recovery Skill

> Extracting and recovering symbols from Windows PE executables.

## Understanding PE Files

```powershell
# Analyze PE structure
Get-Item foo.exe | Format-Hex -Count 256

# Use PEBear or CFF Explorer
# Key sections: .text, .rdata, .data, .rsrc
```

## Symbol Recovery

### 1. Export Table
```powershell
# Use dumpbin or PEParse
dumpbin /EXPORTS foo.dll
```

### 2. Import Table
```powershell
# Find dependencies
dumpbin /IMPORTS foo.exe
```

### 3. Debug Symbols
```powershell
# Check for debug directory
Get-PdbDebugInformation foo.exe

# Look for .pdb files in:
# - Same directory
# - Symbol server
# - Microsoft symbol server
```

## Tools

- **PEBear**: GUI for PE analysis
- **CFF Explorer**: PE editor
- **dumpbin**: MSVC tool
- **pefile**: Python library for PE parsing

## Common Patterns

```python
import pefile

pe = pefile.PE('foo.exe')
print(f"Entry point: {pe.OPTIONAL_HEADER.AddressOfEntryPoint}")
print(f"Image base: {pe.OPTIONAL_HEADER.ImageBase}")

for section in pe.sections:
    print(f"{section.Name.decode()}: {section.VirtualAddress:x}")
```
