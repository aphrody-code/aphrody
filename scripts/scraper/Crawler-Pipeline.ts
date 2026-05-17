import { Database } from "bun:sqlite";
import TurndownService from "turndown";
import { mkdirSync } from "fs";
import { dirname } from "path";

const SITEMAP_URL = Bun.argv[2] || "https://design.google/sitemap.xml/";
const DB_PATH = "C:\\winclean\\var\\data\\winclean.db";
const MAX_PAGES = 5; // Mode test : limite à 5 pages pour la boucle

// Initialisation de la BDD Winclean (Mode WAL strict)
try {
  mkdirSync(dirname(DB_PATH), { recursive: true });
} catch (e) {}
const db = new Database(DB_PATH);
db.exec("PRAGMA journal_mode = WAL;");
db.exec("PRAGMA busy_timeout = 5000;");
db.exec(`
  CREATE TABLE IF NOT EXISTS MaterialDocs (
    url TEXT PRIMARY KEY,
    title TEXT,
    content TEXT,
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
  )
`);

const insertDoc = db.prepare(
  "INSERT OR REPLACE INTO MaterialDocs (url, title, content) VALUES ($url, $title, $content)",
);

// Initialisation de Turndown pour un Markdown de très haute qualité
const turndownService = new TurndownService({ headingStyle: "atx", codeBlockStyle: "fenced" });
turndownService.remove(["script", "style", "nav", "footer", "header", "aside"]);

async function runPipeline() {
  console.log(`[+] Starting Crawler Pipeline on ${SITEMAP_URL}`);

  // Phase 1: Extract Sitemap URLs
  const res = await fetch(SITEMAP_URL, { headers: { "User-Agent": "Winclean-Scraper" } });
  const xml = await res.text();
  const urls = Array.from(xml.matchAll(/<loc>(.*?)<\/loc>/g)).map((m) => m[1]);
  console.log(`[+] Found ${urls.length} URLs in sitemap.`);

  const targetUrls = urls.slice(0, MAX_PAGES);
  console.log(`[+] Processing first ${MAX_PAGES} URLs for the test loop...`);

  // Phase 2 & 3: Extract, Convert, and Store
  for (const url of targetUrls) {
    try {
      console.log(`  -> Scraping: ${url}`);
      const pageRes = await fetch(url, { headers: { "User-Agent": "Winclean-Scraper" } });
      const html = await pageRes.text();

      const titleMatch = html.match(/<title>(.*?)<\/title>/i);
      const title = titleMatch ? titleMatch[1] : "No Title";

      // Isoler le cœur de l'article pour maximiser la qualité du Markdown
      let contentHtml = html;
      const mainMatch =
        html.match(/<main[^>]*>([\s\S]*?)<\/main>/i) ||
        html.match(/<article[^>]*>([\s\S]*?)<\/article>/i);
      if (mainMatch) contentHtml = mainMatch[1];

      const markdown = turndownService.turndown(contentHtml);

      // Injection SQLite
      insertDoc.run({ $url: url, $title: title.trim(), $content: markdown });
      console.log(`     [OK] Saved to SQLite (${markdown.length} bytes)`);

      // Anti-bot throttling
      await new Promise((r) => setTimeout(r, 500));
    } catch (error: any) {
      console.error(`     [ERROR] Failed to process ${url}:`, error.message);
    }
  }

  console.log(`[+] Pipeline Finished. Database updated.`);
}

runPipeline();
