const TARGET_URL = Bun.argv[2] || "https://m3.material.io";

async function fingerprint() {
  console.log(`[+] Fingerprinting frameworks on ${TARGET_URL}...`);
  const res = await fetch(TARGET_URL, { headers: { "User-Agent": "Winclean-Scraper" } });
  const html = await res.text();

  const frameworks: string[] = [];

  if (html.includes("__NEXT_DATA__")) frameworks.push("Next.js");
  if (html.includes("data-reactroot") || html.includes("_reactRoot")) frameworks.push("React");
  if (html.includes("data-vue-router") || html.includes("__NUXT__"))
    frameworks.push("Vue.js / Nuxt");
  if (html.includes('id="___gatsby"')) frameworks.push("Gatsby (React)");
  if (html.includes("svelte-")) frameworks.push("Svelte");
  if (html.includes("ng-version")) frameworks.push("Angular");
  if (html.includes("lit-html") || html.includes("shadowroot") || html.includes("webcomponents-"))
    frameworks.push("Web Components / Lit");
  if (html.includes("js-glue")) frameworks.push("Wasm / Emscripten");

  if (frameworks.length > 0) {
    console.log(`[!] Frameworks Detected: ${frameworks.join(" + ")}`);
  } else {
    console.log(
      "[-] No known SPA framework signatures found. Likely a Static Site Generator or Server-Side Rendered (PHP/Python/Go).",
    );
  }
}

fingerprint();
