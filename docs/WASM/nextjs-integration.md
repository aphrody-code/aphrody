# Next.js 16 + WASM

Source : `vercel/next.js` 16.2+ official docs, verified 2026-05-17.

## Current state of WASM support in Next.js 16

| Bundler | WASM status |
|---------|-------------|
| **Webpack 5** (Next 16 with `--webpack`) | Full `asyncWebAssembly` experiment, `import` of `.wasm` works |
| **Turbopack** (Next 16 default) | WASM async imports **not yet supported** — use Webpack for WASM-heavy apps |

Until Turbopack ships WASM async import, the policy is :
- Apps that depend on WASM modules → opt out of Turbopack until parity.
- Apps without WASM (most) → keep Turbopack for the dev speed.

## Opt-out of Turbopack — `package.json`

```json
{
  "scripts": {
    "dev": "next dev --webpack",
    "build": "next build --webpack",
    "start": "next start"
  }
}
```

## Webpack config — enable WASM async

`next.config.ts` :

```ts
import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  webpack: (config, { isServer }) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    }

    // Required for top-level await in dynamic wasm imports
    config.output = { ...config.output, webassemblyModuleFilename: 'static/wasm/[hash].wasm' }

    return config
  },
}

export default nextConfig
```

## Importing a wasm-bindgen output in a React component

After `wasm-pack build --target bundler` produced `pkg/my_crate.js` and `pkg/my_crate_bg.wasm` :

```tsx
'use client'

import { useEffect, useRef, useState } from 'react'

export default function CounterClient() {
  const [counter, setCounter] = useState<unknown>(null)
  const [value, setValue] = useState(0)

  useEffect(() => {
    let mounted = true
    ;(async () => {
      const wasm = await import('@aphrody-code/my-crate-pkg')   // points to pkg/
      await wasm.default()                                       // bootstrap (target=bundler)
      if (!mounted) return
      const c = new wasm.Counter()
      setCounter(c)
    })()
    return () => { mounted = false }
  }, [])

  if (!counter) return <div>loading…</div>

  return (
    <button onClick={() => setValue((counter as any).increment())}>
      count: {value}
    </button>
  )
}
```

Notes :
- The dynamic `import()` keeps the WASM out of the initial bundle.
- `'use client'` is required — WASM that uses DOM / Canvas / WebGPU can't run in RSC.
- Pin the pkg/ directory as a workspace package, **don't** commit `target/`.

## Server Components with WASM

WASM **can** run in RSC if it's pure compute (no DOM). The Node.js runtime resolves `.wasm` via Webpack's `asyncWebAssembly`. Use `wasm-pack build --target nodejs` for the pkg consumed server-side. The edge runtime supports a smaller subset — verify your wasm-bindgen output doesn't use any Node-only APIs (it shouldn't by default).

## Server Actions + WASM

```ts
'use server'

import { transform } from '@aphrody-code/my-server-wasm-pkg'

export async function processAction(input: string): Promise<string> {
  return transform(input)
}
```

Behavior :
- The `'use server'` module resolves through Next.js's **server-only webpack layer** — WASM imported here stays server-side.
- Caching is fine — the WASM instance is reused across invocations on the same Node process.

## Edge runtime caveats

`export const runtime = 'edge'` constrains what you can ship :
- No filesystem, no Node-specific APIs.
- WASM works if it's `wasm32-unknown-unknown` (not `wasm32-wasi`).
- Bundle limit is **1 MB after gzip** on Vercel Edge. WASM bundles tend to bust this — measure first with `next build` output.

## Bundle analysis

```bash
ANALYZE=true next build --webpack
```

Add `@next/bundle-analyzer` (already in the workspace catalog of `aphrody-code/vps`). Watch the static/wasm chunk — > 500 KB is usually a sign you forgot `wasm-opt -Oz` or pulled in too many `web-sys` features.

## Turbopack roadmap note

Turbopack's WASM support is tracked upstream. When parity lands, the dev script can drop `--webpack`. Monitor [vercel/next.js Turbopack docs](https://github.com/vercel/next.js/blob/canary/docs/01-app/03-api-reference/08-turbopack.mdx) for the green check on async WASM.
