---
name: rpbey-ranking-and-import
description: "How rpbey computes BTS/global + Stardust rankings and imports Challonge tournaments — exact formulas, constants, files, triggers, gotchas"
metadata: 
  node_type: memory
  type: reference
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

rpbey tournament **ranking calculation** (BTS/global + Stardust) and **Challonge import procedure**. Read from the real code 2026-06-04. Related: [[rpbey-perfect-serverless]].

## Data model (Neon Frankfurt, Drizzle)
- `tournaments` (status, weight, categoryId→`tournament_categories.multiplier`, challongeId, id). **Canonical BTS rows: id `bts1`..`bts5`, challongeId `B_TS1`..`B_TS5`** (user-linked). Stardust: challongeId `T_SS1`. The legacy numeric-challongeId rows (`17261774`…, cuid ids) were **deleted 2026-06-04** (backup `~/rpbey-legacy-bts-backup-20260604-1446.json`).
- `tournament_participants` (userId, playerName, challongeParticipantId, seed, finalPlacement, wins, losses, checkedIn). **UNIQUE(tournamentId, challongeParticipantId) was MISSING and added 2026-06-04** — its absence (only `tournament_matches` had it) let `createMany(skipDuplicates)` re-insert `userId:null` copies = the B_TS4 36-dup corruption.
- `tournament_matches` (challongeMatchId UNIQUE(tournamentId,challongeMatchId), round: **>0 winner-bracket, <0 loser-bracket, -100 pool sentinel**, player1/2Name+winnerName, player1/2Id/winnerId=USER ids or null, state, score).
- `global_rankings` (playerName UNIQUE, userId UNIQUE, points, wins, losses, tournamentWins, tournamentsCount) — schema.ts:1932-1972. `stardust_rankings` (NO season col, rank, score, winRate/pointsAverage stored as **text**) + `stardust_bladers` (UNIQUE name, upserted, history JSONB).
- `satr_rankings`/`wb_rankings` are **SEPARATE external imports** (have `season` int, from Google Sheets / local Challonge JSON via syncSatrRanking/syncWbRanking) — NOT the same system as Stardust.
- `ranking_system` = points config. **Table column defaults (participation=5, firstPlace=20…) are legacy/UNUSED**; real values come from `getOrCreateRankingSystem` (participation=500, firstPlace=10000, secondPlace=7000, thirdPlace=5000, top8=500, matchWin=300[unused], matchWinWinner=1000, matchWinLoser=500) or `DEFAULT_STARDUST_CONFIG` (firstPlace=**15000**).

## Import procedure
- **Source**: `apps/web/data/exports/B_TS{n}.json` (processed `{metadata,participants,matches,standings,raw}`), produced by `ChallongeScraper.scrape()` (packages/challonge) via `finalize-tournament.ts`. `B_TS5.raw.json` = raw snake-case stage (importers skip `*.raw.json`). B_TS4/5 ship `standings:[]` → finalPlacement falls back to participant.finalRank; dates are null in exports.
- **Canonical importer `apps/web/scripts/import-bts-tournaments.ts`** — run from **repo root**. `BTS_EDITIONS` pins id=`bts{n}`, challongeId=`B_TS{n}`, loops `[1..5]`. User-link: `normalizeName=raw.split('/')[0].trim()` lowercased → match `users.name/username` + `profiles.bladerName`; **never creates users**; existing userId preserved. finalPlacement = standings.rank ?? finalRank. wins/losses counted from completed matches by challonge participant id. Upsert participant by (tournamentId, challongeParticipantId|playerName|userId); match by (tournamentId, challongeMatchId).
- **Legacy importer `import-bts-to-db.ts` (broken: imports removed `../src/lib/prisma`)** — keyed on numeric String(metadata.id), `createMany(skipDuplicates)` w/o participant unique constraint = dup cause. Run from apps/web. Superseded.
- **Bot live-sync** `apps/bot/src/lib/challonge-sync.ts` `scrapeAndSyncTournament` **auto-creates stub users** (`<clean>@import.bot`). Crons: `LiveTournamentSync` (*/5, status UNDERWAY), `PreTournamentSync` (0 *, date∈[-6h,+24h]). Scrapes via curl-impersonate → **CF-blocked server-side** (cf_clearance is IP-bound); valid Challonge API key is the real auto-sync channel.

## BTS / global ranking calc — `apps/web/src/lib/ranking-recompute.ts` `computeRankings()`
Per participant per tournament: `points = participation(500) + placementBonus + matchWinsWon×matchWinWinner(1000); ×= (category.multiplier ?? tournament.weight ?? 1.0); Math.round`. placementBonus: 1st=10000, 2nd=7000, 3rd=5000, 4–8=top8=500, else 0. **Global path does NOT add matchWinLoser** (BTS JSON path does — see below). wins/losses/tournamentsCount/tournamentWins come from **stored participant.wins/losses columns (NOT recounted)**, only for isFinished (COMPLETE/ARCHIVED); tournamentWins++ if finalPlacement==1; UNDERWAY adds points but not counts. Dedup: playerKey=alias(`participants_map.json`)||normalizeName(NFD/strip-marks/lowercase/`[^a-z0-9]`), then **consolidate rows sharing userId (summed)**; ambiguous name→userId poisoned to null. Reads **ALL finished tournaments, no date/season filter** (BTS exclusion only if enriched JSON present — absent in prod). Tie-break **points DESC, tournamentWins DESC, wins DESC**; read filters points>0. `rebuildGlobalRankings` = DELETE-all+reinsert in a txn + mirror to profiles. **No cron** — triggered by admin `recalculateRankings`, PUT /api/admin/ranking, post-tournament `auto-sync-ranking.ts` (classifyRanking 'global'), or CLI `recompute-rankings.ts`. BTS season JSON path (`actions/bts.ts`, bot `bts-ranking.ts`): same formula but **adds matchWinLoser(500)**; SEASON_MAP s1=[BTS1], s2=[BTS2-5]; `isTrustworthyForPlacements` needs ≥2 distinct finalRanks.

## Stardust ranking calc — `apps/web/src/lib/stardust-sync-bts.ts` (+ bot mirror)
`score = Σ participation(500) + placementBonus(gated) + Σ pointsForWin(round)`; losses score 0. **`pointsForWin(round)`: -100→250 (pool), >0→1000 (winner bracket), <0→500 (loser bracket)** — REPLACES flat matchWinWinner (this is Stardust's key divergence). Placement (gated by `isTrustworthyForPlacements`): firstPlace=**15000**, 2nd=7000, 3rd=5000, 4–8=500. Name canon = `split('/')[0].lowercase` only (**no participants_map alias map**, unlike BTS). winRate/pointsAverage = text (pointsAverage = points/distinct-tournaments). **No season column** (all-time); `replaceStardustRankings` full delete+reinsert; `stardust_bladers` UNIQUE(name) upserted w/ history JSONB. **No cron** (RankingSync commented out + `?skip=stardust`); event-driven. classifyRanking routes by category name: STARDUST→stardust, WILD/WB→wb, SATR/BBT→satr, else→global.

## Gotchas / invariants
- **bot vs web Stardust config diverges**: bot `loadPointsConfig(prisma,{firstPlace:15000})` forces 15000; web uses `ranking_system` row if it exists (could be 10000) else DEFAULT(15000) → can score differently. Two near-identical `buildStardustRankings` copies (web inlines, bot delegates to `ranking-provider.ts`) — edits must be mirrored.
- Global wins/losses are read from stored columns → stale if a tournament wasn't re-imported.
- Stardust file-header comment ("matchWinWinner/Loser") is **stale/misleading** — code uses pointsForWin(round).
- `-100` pool sentinel written by `scripts/scrape-pool-matches.ts`; wrong round value → pool wins mis-weighted as loser-bracket.
- PUT /api/admin/ranking body field `matchWin` is the legacy unused column, not `matchWinWinner` — editing it doesn't change the formula.
