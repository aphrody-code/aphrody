# C# NativeAOT Development Skill

> High-performance C# development with NativeAOT compilation.

## When to Use

- Building performance-critical applications
- Creating standalone executables
- Windows native tool development
- MCP server development

## Key Requirements

### 1. No Reflection
NativeAOT trims reflection metadata. Use:
- System.Text.Json with source generators (NOT JsonSerializer)
- Static code analysis instead of runtime reflection
- Manually register what you need

### 2. Source Generators
```csharp
// ✅ Good: Source generator
[JsonSerializable(typeof(MyDto))]
public partial class MyDtoContext : JsonSerializerContext { }

// ❌ Bad: Runtime serialization
var json = JsonSerializer.Serialize(obj); // Will fail at runtime
```

### 3. AOT-Compatible APIs
```csharp
// ✅ Use these
System.Text.Json
MemoryMarshal
Unsafe.As

// ❌ Avoid these
Newtonsoft.Json
Json.NET
dynamic
```

## Build Command

```powershell
# NativeAOT publish
dotnet publish -c Release -r win-x64 --self-contained

# Trimming enabled
dotnet publish -c Release -r win-x64 --self-contained -p:PublishTrimmed=true
```

## WinClean Integration

- Output native executables to `C:\winclean\bin\`
- MCP servers go to `C:\winclean\apps\mcp\`
- Use PowerShell for interop
