# Python Performance Optimization Skill

> Optimizing Python code for production workloads.

## Key Strategies

### 1. Profiling First
```python
# Use cProfile
python -m cProfile -s cumulative script.py

# Use py-spy
py-spy record -o profile.svg -- python script.py
```

### 2. Data Structures
```python
# ✅ Use for lookups
d = {}  # dict: O(1)
s = set()  # set: O(1)

# ❌ Avoid for large data
l = []  # list: O(n)
```

### 3. NumPy/Numba
```python
import numpy as np
import numba

@numba.jit
def fast_sum(arr):
    total = 0.0
    for i in range(len(arr)):
        total += arr[i]
    return total
```

### 4. Async for I/O
```python
import asyncio

async def fetch_all(urls):
    async with aiohttp.ClientSession() as session:
        tasks = [session.get(url) for url in urls]
        return await asyncio.gather(*tasks)
```

## Package Management

```powershell
# Use uv (fastest)
uv pip install package
uv pip install -r requirements.txt

# Or pip (legacy)
pip install package
```

## WinClean Rules

- Use **uv** for package management (NOT pip for new projects)
- Store volatile data in `C:\winclean\var\json\`
- Profile before optimizing
