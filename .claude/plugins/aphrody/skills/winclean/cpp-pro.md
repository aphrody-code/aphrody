# C++ Professional Development Skill

> Modern C++20 development with best practices for Windows.

## Modern C++ Features

### Concepts & Requires
```cpp
template <typename T>
concept Addable = requires(T a, T b) {
    a + b;
};

template <Addable T>
T add(T a, T b) { return a + b; }
```

### Ranges
```cpp
#include <ranges>
#include <algorithm>

auto result = std::views::filter(v, [](auto x) { return x > 0; })
                    | std::views::transform([](auto x) { return x * 2; })
                    | std::ranges::to<std::vector>();
```

### Span
```cpp
#include <span>

void process(std::span<const int> data) {
    for (auto x : data) { /* ... */ }
}
```

## Windows-Specific

### Microsoft Extensions
```cpp
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

// Safe string functions
StringCchCopyA(dst, dstSize, src);
```

### Memory Management
```cpp
// Use Windows Heap APIs for performance
HANDLE hHeap = GetProcessHeap();
void* p = HeapAlloc(hHeap, HEAP_ZERO_MEMORY, size);

// Or use std::unique_ptr with custom deleter
auto p = std::unique_ptr<T[], LocalFreeDeleter>(
    static_cast<T*>(LocalAlloc(LPTR, size))
);
```

## Toolchain

- **Compiler**: clang-cl (LLVM) or MSVC
- **Build**: CMake + Ninja
- **Linter**: clang-tidy

## WinClean Rules

- Use C++20 features
- Prefer std:: over raw Windows APIs where possible
- Enable warnings and treat as errors
