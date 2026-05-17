// SPDX-License-Identifier: Apache-2.0

/**
 * Next.js config tuned for the aphrody fork (16.3.0-canary.2).
 *
 * Notes:
 *   - `transpilePackages` is needed because we import raw .ts source from
 *     `@aphrody/gemini-live-aphrody` and `@aphrody-code/ui`.
 *   - `turbopack` (was `experimental.turbo` in 15.x) is the Next.js 16
 *     stable name. We pass an empty object to opt in with defaults so
 *     `next dev --turbo` is wired without extra rule plumbing.
 *   - We do NOT set `output: 'standalone'` here because dev mode does not
 *     produce that artifact; switching it on for `next build` belongs in
 *     a CI-specific config.
 *
 * `NextConfig` is declared in `next/dist/types` which only exists once the
 * fork is built; we use a structural local type so this file compiles
 * against either a built or unbuilt fork.
 */
interface NextConfig {
  reactStrictMode?: boolean;
  transpilePackages?: string[];
  turbopack?: Record<string, unknown>;
  webpack?: (config: { module: { rules: unknown[] } }) => { module: { rules: unknown[] } };
}

const nextConfig: NextConfig = {
  reactStrictMode: true,
  transpilePackages: [
    "@aphrody/gemini-live-aphrody",
    "@aphrody-code/ui",
    "@material/web",
    "@lit/reactive-element",
    "lit",
    "lit-html",
    "lit-element",
  ],
  turbopack: {
    rules: {
      "*.wgsl": {
        loaders: ["raw-loader"],
        as: "*.js",
      },
    },
  },
  webpack: (config: { module: { rules: unknown[] } }) => {
    config.module.rules.push({
      test: /\.wgsl$/,
      type: "asset/source",
    });
    return config;
  },
};

export default nextConfig;
