// Bun-side: the HTML entry imported by the Bun.serve fullstack server.
declare module "*.html" {
  const html: import("bun").HTMLBundle;
  export default html;
}

// The bxc `/google` barrel transitively reaches an anti-detection helper that
// `import`s the optional `patchright` peer (only ever loaded at runtime, and
// never on the path qa-gemini.ts exercises). It is not installed here, so give
// tsc an ambient stub purely to keep the QA harness's typecheck honest without
// pulling a heavy browser-automation peer into the showcase.
declare module "patchright" {
  // `any` (not `unknown`) so the un-typed helper's `page.*` calls inside the
  // absent peer's consumers don't surface errors in code we don't ship.

  export type Page = any;

  export type Browser = any;

  export type BrowserContext = any;

  const patchright: any;
  export default patchright;
}
