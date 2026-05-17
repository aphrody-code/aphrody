# aphrody-translate

CLI Rust qui parcourt un projet, extrait tous les commentaires source, retire les marqueurs IA et les émoji, traduit en français, puis applique le style **Aphrody** — prose sobre, impersonnelle, centrée sur le code.

## Conception

Quatre étages successifs, totalement déterministes :

```
walk (ignore-aware)
  ↓
extract  (regex purs Rust, par langage)
  ↓
ai_patterns::classify → Drop | Scrub | Keep
  ↓
translate (MyMemory API gratuite + cache JSON disque)
  ↓
aphrodify (rewrite stylistique)
  ↓
write (in-place ou dry-run)
```

Aucun appel à un modèle IA pour transformer le texte : seule la **traduction** passe par MyMemory (qui fait du statistical MT, pas du LLM). Tout le reste est rule-based.

## Pourquoi pas tree-sitter

V1 reste sur regex pour rester pur Rust cross-platform (Linux + Windows) sans dépendance native C par grammaire. Le compromis est fonctionnel : >95 % des commentaires hors-chaîne sont capturés correctement. Tree-sitter sera ajouté en V2 pour gérer les commentaires imbriqués dans des chaînes (rare).

## Langages supportés

| Langage | Styles de commentaires |
|---------|------------------------|
| Rust | `///`, `/** */`, `//`, `/* */` |
| TypeScript / JavaScript | `/** */`, `/* */`, `//` |
| Python | `# `, `""" """` |
| Go | `//`, `/* */` |
| C / C++ | `//`, `/* */` |
| Shell | `#` |
| TOML | `#` |
| Markdown | `<!-- -->` |

## Utilisation

```bash
# Dry-run (par défaut)
aphrody-translate --root .

# Tout réécrire en place
aphrody-translate --root . --in-place

# Limiter aux fichiers Rust
aphrody-translate --root . --in-place --languages rust

# Scrub + aphrodify sans appel réseau
aphrody-translate --root . --no-translate --in-place

# Quota élargi (50 000 mots/jour) en passant un email noreply
aphrody-translate --root . --in-place \
    --contact-email 37252373+aphrody-code@users.noreply.github.com
```

## Cache

Toutes les traductions atterrissent dans `<root>/.aphrody-translate-cache.json`
(BTreeMap sha256 → traduction). Une ré-exécution sur les mêmes commentaires est
instantanée et sans réseau. Le fichier est sûr à commiter : il ne contient pas
de secret, seulement des phrases traduites.

## Règles ai_patterns — extraits

**Drop** (la ligne entière disparaît) :
- `🤖 Generated with [Claude Code](...)`
- `Co-Authored-By: Claude|Gemini|Copilot|GPT`
- `Written/Made/Assisted by AI`

**Scrub** (le fragment est retiré, le reste est gardé) :
- `with help from Claude|GPT|Gemini|Copilot|...`
- Tout caractère émoji (plages U+1F300..U+1FAFF, U+2600..U+27BF, etc.)

**Keep** sinon.

## Règles aphrodify — extraits

- Préfixes vides : `Note:`, `Nota bene:`, `Remarque:` → retirés
- TODO vague (`TODO: implement|finish|complete|do`) → ligne supprimée
- Filler conversationnel : `Let's`, `Let me`, `I'll`, `We will`, `Voyons`, `D'abord` → retirés
- Première personne FR : `je`, `j'ai`, `nous`, `notre`, `mon`, `ma`, `mes` → impersonnel
- Première personne EN : `I`, `we`, `our`, `my` → impersonnel
- Espaces multiples normalisés, ponctuation orpheline collée
- Première lettre capitalisée, point final ajouté si manquant

## Tests

```bash
cargo test -p aphrody-translate
```

13 tests unitaires couvrent extract / ai_patterns / aphrodify avec exemples FR
et EN.

## Limites assumées

- Heuristique `is_french` : approximative, basée sur accents et mots-outils. Un
  faux négatif coûte un aller-retour réseau MyMemory ; un faux positif laisse un
  commentaire anglais non traduit. Acceptable pour V1.
- MyMemory peut rate-limit (HTTP 429). Le client tombe alors en mode passthrough
  (texte original conservé) avec warning dans le log.
- Réécriture des chaînes de caractères : volontairement non touchée. Seuls les
  vrais commentaires sont modifiés.

## License

Apache-2.0. Voir `LICENSE` à la racine du workspace.
