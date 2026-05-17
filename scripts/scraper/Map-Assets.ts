// SPDX-License-Identifier: Apache-2.0
const TARGET_URL = Bun.argv[2] || "https://m3.material.io";

async function mapAssets() {
  console.log(`[+] Mapping assets from ${TARGET_URL} using HTMLRewriter...`);
  const res = await fetch(TARGET_URL, {
    headers: { "User-Agent": "Mozilla/5.0 Winclean-Scraper" },
  });

  const images = new Set<string>();
  const styles = new Set<string>();
  const scripts = new Set<string>();

  const rewriter = new HTMLRewriter()
    .on("img", {
      element(el) {
        // Capture des images lazy-loadées (data-src) fréquentes sur les sites Google
        const src =
          el.getAttribute("src") || el.getAttribute("data-src") || el.getAttribute("srcset");
        if (src) images.add(src);
      },
    })
    .on("link", {
      element(el) {
        const rel = el.getAttribute("rel");
        if (rel === "stylesheet" || rel === "preload") {
          const href = el.getAttribute("href");
          if (href) styles.add(href);
        }
      },
    })
    .on("script", {
      element(el) {
        const src = el.getAttribute("src");
        if (src) scripts.add(src);
      },
    });

  await rewriter.transform(res).text();

  console.log(`[+] Found ${images.size} unique images/graphics`);
  console.log(`[+] Found ${styles.size} unique stylesheets/preloads`);
  console.log(`[+] Found ${scripts.size} unique scripts`);
}

mapAssets();
