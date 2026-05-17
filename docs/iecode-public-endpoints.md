# IEVR / iecode — endpoints publics catalog

Probed 2026-05-17 — actionnable pour aphrody CLI + winclean iecode-web. Pair avec [`docs/WINCLEAN-AUDIT.md`](WINCLEAN-AUDIT.md) (audit sibling repo) et `C:\winclean\ai.json` (CollaborationManifest A2A v0.4).

## 1. azalee.rosegriffon.fr — prod Next.js site

Hébergé sur **Vercel**, tech stack confirmé via headers + chunks :

| Aspect | Valeur |
|---|---|
| Runtime | Next.js + **Turbopack** (chunk `turbopack-0dvncaisiclna.js`) |
| Deployment ID | `dpl_DtvyL3PPBcru3dQhegRomVRvJV17` |
| Server | Vercel |
| CSP `connect-src` | `'self' https://www.google-analytics.com https://www.googletagmanager.com https://*.supabase.co wss://*.supabase.co` |
| CSP `img-src` whitelist | `https://dxi4wb638ujep.cloudfront.net`, `https://*.inazuma.jp`, `https://*.supabase.co`, googleusercontent, discordapp, twimg |
| Fonts | Google Sans Flex Variable + Humane Regular |
| Frame-ancestors | `'none'` (no iframe embedding) |

### Endpoints HTTP testés

| Méthode | Path | Status | Notes |
|---|---|---|---|
| GET | `/` | 200 | SPA Next |
| GET | `/robots.txt` | 200 | (à recharger pour contenu) |
| GET | `/sitemap.xml` | (probed) | — |
| **POST** | **`/api/graphql`** | **200** | **Endpoint live**. Introspection (`__schema { ... }`) **bloquée** → réponse `{"errors":[{"message":"Unexpected error","extensions":{"code":"INTERNAL_SERVER_ERROR"}}]}`. Pattern : Apollo Server `introspection: process.env.NODE_ENV !== 'production'`. |
| GET | `/api/openapi` | 200 SPA catch-all | pas d'OpenAPI prod (le generator vit côté inagle source, non exposé) |
| GET | `/api/openapi.json` | 200 SPA catch-all | idem |
| GET | `/api/trpc` | (probed) | (loop pending, à compléter) |

**Pour découvrir le GraphQL schema** : sans introspection prod, deux voies :
1. Reverse-engineer depuis les requêtes faites par le client Next.js — capturer via DevTools Network, ou parser les chunks JS (les opérations GraphQL sont inlinées par Next).
2. Lire le code source côté inagle — `@rosegriffon/inagle` v2.0.1 expose `characters/api.ts`, `basara/api.ts`, `api/drops.ts` qui forment probablement la base de l'API.

## 2. inagle (`@rosegriffon/inagle`) — game data API source

| Aspect | Valeur |
|---|---|
| Path local | `C:/worktree/vps/packages/inagle/` |
| Version | 2.0.1 |
| Licence | MIT |
| Description | "Universal Game Data API for Inazuma Eleven: Victory Road — parsers, types, and tools for IEVR game data" |
| Runtime préféré | **Bun** (avec fallback dist pour Node/autres) |
| HTTP adapter | `src/adapters/hono.ts` (Hono framework — supporte `@hono/zod-openapi` plugin) |
| CLI bin | `inagle` |
| Scripts notables | `bun src/scripts/serve-data.ts` (server), `bun src/scripts/generate-entries.ts`, `bun src/cli-push.ts` |
| Keywords | inazuma-eleven, victory-road, ievr, game-data, parser, **cfg-bin**, **iecode**, **zukan** |

### Modules src/

- `basara/{api,index,types}.ts` — Basara crawl (azalée upstream ?)
- `characters/{api,evolution,index,mapper,mapper-v3,types}.ts` — character API
- `api/drops.ts` — drop tables
- `analysis/{index,matcher}.ts` — heuristics
- `cli-commands.ts` — CLI dispatch

Le CDN azalee whitelist `dxi4wb638ujep.cloudfront.net` + `*.inazuma.jp` + `*.supabase.co` → assets game probably servis depuis CloudFront + Supabase est la DB backing.

## 3. Steam Web APIs publiques (no auth)

| API | URL | Données |
|---|---|---|
| **Store appdetails** | `https://store.steampowered.com/api/appdetails?appids=2799860&l=french` | game type, name, required_age, controller_support, DLC list, detailed_description FR, screenshots, languages, genres, categories |
| **SteamSpy appdetails** | `https://steamspy.com/api.php?request=appdetails&appid=2799860` | dev=LEVEL5, pub=LEVEL5, price=6999, languages=9, owners="0..20K", **genre=RPG+Sports** |
| Store appdetails (DLC) | `?appids=3550790&l=french` | "INAZUMA ELEVEN : Victory Road - Édition améliorée (édition deluxe)" |

**Key data extracted** :
- AppID **2799860** (game), DLC **3550790** (deluxe)
- Dev/Pub : **LEVEL5 Inc.**
- Price : **6999** (€69.99)
- Languages : EN, FR, IT, DE, ES, **PT-BR**, **Zh-CN**, **Zh-TW**, JP
- Genre : RPG + Sports
- Story : 25 ans après Inazuma Eleven 1, protagoniste **Destin Billows** (collège Nagumohara), **Harper Evans** (collège Raimon) — *les noms FR diffèrent du JP* (cf. iecode-zukan-names task open dans winclean ai.json)
- **5400+ joueurs** récupérables (chronology mode — replays des matchs historiques)
- Mode "Station Kiz…" (description Steam truncated)

## 4. zukan.inazuma.jp — official zukan

| Path | Status | Notes |
|---|---|---|
| `/robots.txt` | 200 (HTML doctype) | SPA returning index.html on robots → no robots policy |
| `/api`, `/graphql`, `/api/graphql`, `/api/graphql/v1` | (probed by other instance) | endpoints à compléter |

## 5. Sources inazuma annexes worth probing (TODO)

- `*.inazuma.jp` whitelisted in azalee CSP → potentiel sub-domains (assets, media, official-pro)
- CloudFront `dxi4wb638ujep.cloudfront.net` — origin S3 likely
- `*.supabase.co` → l'instance Supabase backing azalee (auth + realtime + storage)
- Discord/Twitter mentions dans CSP → community feeds

## 6. Recommandations actions

| Pour | Action |
|---|---|
| Découvrir le GraphQL schema en prod | Capturer requêtes via Edge DevTools + reverse depuis les chunks Next OU lire `inagle/src/characters/api.ts` + `basara/api.ts` (source de vérité) |
| Aphrody CLI consume inagle | `inagle` est Bun-first MIT. Wrapper Rust possible via `cargo run -- iecode chars <q>` qui spawn `bun --cwd <inagle> src/cli.ts chars <q>` |
| iecode-web exposer OpenAPI | Si winclean veut, ajouter `@hono/zod-openapi` aux routes Hono dans inagle — generator déjà mentionné par instance peer |
| Mirror CDN assets sans rate-limit | Cache local OPFS via Bun fetch → cloudfront origin, respecter rate limits |
| Cross-validate iecode-chars.db | Comparer les 6448 chars locaux avec le count Steam Store "5400+ players" — écart ~1000 (probably DLC chars or future content) |
