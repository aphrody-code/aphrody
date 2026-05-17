const TARGET_URL = Bun.argv[2] || "https://m3.material.io";

async function inspectTarget() {
  console.log(`[+] Inspecting ${TARGET_URL}...`);
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 5000); // 5s timeout

    const res = await fetch(TARGET_URL, {
      signal: controller.signal,
      headers: {
        "User-Agent":
          "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Winclean-Scraper/1.0",
      },
    });
    clearTimeout(timeoutId);

    console.log(`[+] Status: ${res.status}`);
    const server = res.headers.get("server") || res.headers.get("x-powered-by") || "Unknown";
    console.log(`[+] Server/CDN/Stack: ${server}`);

    const robotsRes = await fetch(`${new URL(TARGET_URL).origin}/robots.txt`);
    if (robotsRes.ok) {
      const robotsTxt = await robotsRes.text();
      const sitemaps = Array.from(robotsTxt.matchAll(/Sitemap:\s*(.+)/gi)).map((m) => m[1]);
      console.log(`[+] Sitemaps found:`, sitemaps.length > 0 ? sitemaps : "None");
    } else {
      console.log(`[-] Robots.txt not found (Status: ${robotsRes.status})`);
    }
  } catch (error: any) {
    console.error(`[-] Inspection failed:`, error.message);
  }
}

inspectTarget();
