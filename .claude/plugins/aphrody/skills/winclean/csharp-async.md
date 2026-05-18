# C# Async/Await Performance Skill

> Optimizing async code for maximum throughput and minimum latency.

## Common Anti-Patterns

### 1. Async Overhead
```csharp
// ❌ Bad: Async without await
public async Task<string> GetData() {
    return await _service.GetAsync();
}

// ✅ Better: Return Task directly
public Task<string> GetData() => _service.GetAsync();
```

### 2. Blocking on Async
```csharp
// ❌ Bad: .Result blocks thread
var result = data.GetAsync().Result;

// ✅ Good: await asynchronously
var result = await data.GetAsync();
```

### 3. Fire and Forget
```csharp
// ❌ Bad: Unobserved exception
_ = _service.ProcessAsync(); // Exceptions lost!

// ✅ Good: Handle or log
try {
    await _service.ProcessAsync();
} catch (Exception ex) {
    _logger.LogError(ex, "Process failed");
}
```

## Performance Tips

1. **Use ValueTask** for already-completed operations
2. **ConfigureAwait(false)** in library code
3. **Avoid** capturing context in hot paths
4. **Pool** async state machines for high-frequency operations

## Channel Pattern

```csharp
using System.Threading.Channels;

var channel = Channel.CreateBounded<Event>(100);

// Producer
await channel.Writer.WriteAsync(new Event { });

// Consumer
while (await channel.Reader.WaitToReadAsync()) {
    while (channel.Reader.TryRead(out var e)) {
        Process(e);
    }
}
```
