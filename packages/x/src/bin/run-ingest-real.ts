// SPDX-License-Identifier: Apache-2.0
import { ingestBeybladeData } from "../db/ingest";
import { Store } from "../db/store";

import { join } from "node:path";
import { existsSync, readdirSync } from "node:fs";

function findBeybladeData(): string {
  const home = Bun.env.HOME || "/home/ubuntu";
  const cliPath = process.argv[2];
  if (cliPath) return cliPath;

  const currentScratch = join(home, ".gemini/antigravity-cli/brain/dd605dfb-b2ce-4b1b-b188-fef48150a92c/scratch/beyblade_data.json");
  if (existsSync(currentScratch)) return currentScratch;

  const brainDir = join(home, ".gemini/antigravity-cli/brain");
  if (existsSync(brainDir)) {
    try {
      const dirs = readdirSync(brainDir);
      for (const d of dirs) {
        const candidate = join(brainDir, d, "scratch", "beyblade_data.json");
        if (existsSync(candidate)) return candidate;
      }
    } catch (_) {}
  }

  return join(home, ".gemini/antigravity-cli/brain/915df5ef-84a3-4d37-a2c1-92f6e24b5e5c/scratch/beyblade_data.json");
}

async function main() {
  const filePath = findBeybladeData();
  console.log(`Loading SQLite Store...`);
  const store = new Store(); // uses default path ~/.aphrody/x-store.sqlite
  
  console.log(`Starting ingestion of ${filePath}...`);
  try {
    const stats = await ingestBeybladeData(filePath, store);
    console.log(`Ingestion completed successfully!`);
    console.log(`- Tweets Ingested: ${stats.tweetsIngested}`);
    console.log(`- Users Ingested: ${stats.usersIngested}`);
    console.log(`- Communities Ingested: ${stats.communitiesIngested}`);
    
    const dbStats = store.stats();
    console.log(`Database Current Stats:`);
    console.log(`- Path: ${dbStats.path}`);
    console.log(`- Total Tweets: ${dbStats.tweets}`);
    console.log(`- Total Users: ${dbStats.users}`);
    console.log(`- Total Edges: ${dbStats.edges}`);
    console.log(`- Total Follows: ${dbStats.follows}`);
  } catch (err: any) {
    console.error(`Ingestion failed: ${err.message}`);
  } finally {
    store.close();
  }
}

main().catch(console.error);
