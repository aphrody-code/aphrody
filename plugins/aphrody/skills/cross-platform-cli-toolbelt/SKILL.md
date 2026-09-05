---
name: cross-platform-cli-toolbelt
description: "Rust CLI replacements for slow GNU coreutils / busybox / PowerShell — same binary on Linux/Windows/macOS. Use whenever picking a shell command, writing a script, or benchmarking. Maps grep→rg, find→fd, sed→sd, ls→eza, cat→bat, du→dust, ps→procs, htop→btm, etc. with installed-state inventory."
version: 1.0.0
source: 32blog.com + sts10/rust-command-line-utilities + repotoire.com 2026-05-19
---

# Cross-platform CLI toolbelt — May 2026

Mode `/goal` permanent : décider seul, ne pas s'arrêter avant complétion.

Toolbelt Rust de remplacement des classics GNU/BusyBox/PowerShell. Tous shippés
**single static binary**, **identique Linux/Windows/macOS**, **5–50× plus rapide**,
**respect `.gitignore`** par défaut.

---

## 0. Pourquoi pas GNU/PowerShell ?

| Problème classic | Conséquence | Fix Rust |
|---|---|---|
| `find . -name '*.rs' -type f` | parse-time + syntaxe mémorisable | `fd -e rs` (5–10× faster, parallel walk) |
| `grep -r --include='*.rs' pat .` | scan toutes les `.gitignore`-ignored | `rg pat -t rust` (respecte gitignore) |
| `sed -i 's/foo/bar/g' file` | escaping cryptique | `sd 'foo' 'bar' file` (regex Rust syntaxe normale) |
| `ls -la --color` | sort/permissions/git-status off | `eza -lag --git --icons` |
| `cat file.rs` | pas highlight | `bat file.rs` (syntect highlight, paging, git) |
| `du -sh */ \| sort -h \| head` | 4 commandes pour 1 résultat | `dust` (tree viz instantanée) |
| `ps aux \| grep cargo` | output texte non-structuré | `procs cargo` (colored, tree, port mapping) |
| `htop` | macOS-only sans portage Win | `btm` (top + CPU/mem/net/disk graphs) |
| `time cmd` | pas de statistical analysis | `hyperfine 'cmd'` (warmup, stddev, JSON) |
| `make target` | Makefile DSL complexe | `just target` (justfile syntaxe simple) |
| `cd ../foo/bar/baz` | frappe répétée | `z baz` (zoxide, frecency match) |
| `git diff` | output mono | `delta` (side-by-side, syntax highlight) |
| `wc -l **/*.rs` | une seule lang | `tokei` (toutes langs, JSON output) |
| `inotifywait \| while; do …` | Linux-only | `watchexec --watch src cmd` (cross-platform) |
| `cp/mv/rm` | pas de progress, pas safe | `xcp` (progress bar) + `trash` (safe rm) |

---

## 1. Inventaire installé (à détecter sur le host courant)

Détecter ce qui est présent avant de scripter (ne pas supposer un chemin codé en dur) :

```bash
# Linux/macOS
for t in rg fd sd eza bat dust btm procs hyperfine just zoxide delta tokei watchexec starship; do
  command -v "$t" >/dev/null 2>&1 && echo "$t ✅ $(command -v "$t")" || echo "$t ❌"
done
```
```powershell
# Windows (PowerShell)
'rg','fd','sd','eza','bat','dust','btm','procs','hyperfine','just','zoxide','delta','tokei','watchexec','starship' |
  ForEach-Object { $p = (Get-Command $_ -ErrorAction SilentlyContinue).Source; "$_ $(if($p){"✅ $p"}else{'❌'})" }
```

### Bulk install one-shot (reproductibilité fresh host)

```bash
# si cargo-binstall absent :
cargo install cargo-binstall

# 8 outils manquants par défaut (les 7 autres : rg via winget, fd/zoxide/starship via scoop)
cargo binstall --no-confirm sd bat du-dust git-delta bottom procs tokei watchexec-cli
```

Sur Windows alternative (déjà partiellement utilisé) : `scoop install sd bat dust delta bottom procs tokei watchexec`.

Sur Linux Ubuntu 26.04 : `apt install ripgrep fd-find bat eza` + `cargo binstall …` pour le reste (Ubuntu 26.04 a uutils coreutils en default, cf. §8).

---

## 2. Tableau de référence — Rust CLI 2026

| Outil | Remplace | Install (cargo) | Latency vs classic | Killer feature |
|---|---|---|---|---|
| **ripgrep** (`rg`) | `grep -r` | `ripgrep` | 5–20× | `.gitignore`-aware, multi-line, `--json` output |
| **fd-find** (`fd`) | `find` | `fd-find` | 5–10× | sane defaults, parallel walk, `--exec` parallèle |
| **sd** | `sed` | `sd` | 2–5× | regex Rust standard, no `-i ''` hack macOS |
| **eza** | `ls` | `eza` | comparable | git status inline, icons, tree mode |
| **bat** | `cat` | `bat` | comparable | syntect highlight, line numbers, paging auto |
| **dust** | `du` | `du-dust` | 2–5× | tree viz, colors, instant `--depth N` |
| **bottom** (`btm`) | `htop` / `top` | `bottom` | n/a | cross-platform (Win!), CPU/mem/net/disk graphs |
| **procs** | `ps` | `procs` | comparable | colored, tree, port-process map, `--watch` |
| **hyperfine** | `time` | `hyperfine` | n/a | warmup, statistical, Markdown/JSON export |
| **just** | `make` | `just` | n/a | recipes, env vars, no `\n` quirks |
| **zoxide** (`z`) | `cd` + `autojump` | `zoxide` | 10× vs autojump | frecency algorithm, init zsh/bash/pwsh/fish |
| **delta** | git pager | `git-delta` | comparable | side-by-side, syntax-highlight, line-num |
| **tokei** | `cloc` / `sloccount` | `tokei` | 100× cloc | JSON/YAML output, all langs |
| **watchexec** | `inotifywait` / `fswatch` | `watchexec-cli` | n/a | cross-platform, gitignore-aware, debounce |
| **starship** | bash/zsh prompt themes | `starship` | n/a | universal prompt, all shells incl pwsh |
| **bottom** alt **gpustat** | `nvidia-smi` watch | `bottom --no_gpu false` | n/a | inclus dans btm |
| **xcp** | `cp` | `xcp` | 2× | progress bar, parallel chunks |
| **trash** | `rm` | `trash-cli` | n/a | safe rm (XDG trash spec) |
| **broot** | `tree` + `cd` | `broot` | n/a | TUI navigator, fuzzy, du integration |
| **lsd** | `ls` (alt eza) | `lsd` | comparable | icons, git, plus simple qu'eza |
| **gitui** | `tig` / `lazygit` | `gitui` | n/a | TUI git complet, mouse support |
| **mprocs** | `tmux` (simple) | `mprocs` | n/a | run multiple cmds in TUI panes |
| **xh** | `curl` / `httpie` | `xh` | 5× httpie | API REST friendly, JSON colored |
| **dog** | `dig` | `dogdns` | comparable | colored, sane defaults, DoH/DoT |
| **gping** | `ping` | `gping` | n/a | graph TUI multi-host |
| **bandwhich** | `iftop` / `nettop` | `bandwhich` | n/a | per-process bandwidth, Linux/macOS/Win |
| **choose** | `cut` + `awk` | `choose` | 2× | syntaxe `choose 1:3` Pythonic |
| **jql** / **jaq** | `jq` | `jql` / `jaq` | 2–5× jq | Rust impl, jaq quasi-parité jq syntaxe |
| **xsv** / **qsv** | `csvkit` | `xsv` / `qsv` | 10× | streaming, indexed select, joins |
| **miller** (`mlr`) | csv/tsv/json swiss | `miller` | comparable | (Go-based, mention pour contexte) |

---

## 3. Mapping bash → Rust (cheat-sheet)

```bash
# search content
grep -r "TODO"           →  rg TODO
grep -ri "fail"          →  rg -i fail
grep --include="*.rs"    →  rg -t rust
grep -A 3 -B 1 "panic"   →  rg -A 3 -B 1 panic

# find files
find . -name "*.rs"      →  fd -e rs
find . -type d -name x   →  fd -t d x
find . -newer file       →  fd --changed-within 1h
find . -exec wc -l {} +  →  fd -e rs -x wc -l    (parallel par défaut)

# substitute
sed -i 's/foo/bar/g' f   →  sd foo bar f                 (in-place par défaut)
sed -E 's/(\d+)/[\1]/'   →  sd '(\d+)' '[$1]'             (PCRE Rust)

# list
ls -la --color           →  eza -la
ls -la --git --tree      →  eza -la --git -T
ls -la --sort=size       →  eza -la --sort=size

# view
cat file.rs              →  bat file.rs
cat file.json | jq .     →  bat file.json --language=json    (ou : jaq . < file.json | bat -l json)

# disk
du -sh */                →  dust -d 1
du -h | sort -h | tail   →  dust  (déjà trié)

# process
ps aux | grep cargo      →  procs cargo
top                      →  btm
htop                     →  btm --basic
nvidia-smi               →  btm  (--no_gpu false)

# benchmark
time ./bin               →  hyperfine './bin'
time a; time b           →  hyperfine './a' './b'  (warmup auto)

# make
make build               →  just build
make -j8 test            →  just test    (just gère parallel via recipe)

# git
git diff                 →  git diff       (avec [core] pager=delta dans gitconfig)
git log -p               →  same

# count lines
wc -l **/*.rs            →  tokei
cloc .                   →  tokei --output json

# watch
inotifywait -m src       →  watchexec --watch src 'cargo check'
make test on change      →  watchexec -e rs 'cargo test'

# HTTP
curl -X POST -H 'CT: app/json' -d '{"x":1}' url    →  xh post url x=1
```

---

## 4. Mapping PowerShell → Rust (cheat-sheet)

```powershell
# pwsh                                  →  rust
Get-ChildItem -Recurse -Filter *.rs     →  fd -e rs
Get-ChildItem | Where-Object {…}        →  fd …
Select-String "TODO" -Path *.rs         →  rg TODO -g '*.rs'
(Get-Content f) -replace 'a','b' | Set-Content f    →  sd a b f
Get-Content file.rs                     →  bat file.rs
Get-ChildItem | Sort-Object Length      →  eza -la --sort=size
Get-Process | Where Name -like '*cargo*' →  procs cargo
Measure-Command { ./bin }               →  hyperfine './bin'
Get-Volume                              →  duf  (Rust : `dust` couvre file-tree, `duf` Go disk-summary)
Set-Location ..\foo\bar                 →  z bar    (zoxide init dans $PROFILE)
ipconfig /all                           →  (Windows-only) `ipconfig` natif ou MCP Win32 dédié
Get-NetTCPConnection                    →  bandwhich  (ou un MCP Win32 list_tcp_connections)
```

**⚠ Important (Windows-only)** : pour le registre, services, scheduled tasks,
process memory, DWM, window mgmt, préférer un MCP Win32 dédié (P/Invoke direct)
plutôt que des tools Rust génériques.

---

## 5. Configuration globale recommandée

### `~/.gitconfig` — delta

```ini
[core]
    pager = delta
[interactive]
    diffFilter = delta --color-only
[delta]
    navigate = true
    line-numbers = true
    syntax-theme = Monokai Extended
[merge]
    conflictStyle = zdiff3
```

### `~/.bashrc` ou `~/.zshrc` — aliases + zoxide + starship

```bash
# Drop-in replacements
alias ls='eza --icons --git'
alias ll='eza -la --icons --git'
alias cat='bat --paging=never'
alias find='fd'
alias grep='rg'
alias ps='procs'
alias top='btm'
alias du='dust'

# Frecency cd
eval "$(zoxide init bash)"   # ou zsh / fish

# Universal prompt
eval "$(starship init bash)"
```

### `$PROFILE` (PowerShell) — équivalent

```powershell
Set-Alias ll eza
Set-Alias cat bat
Set-Alias grep rg
Set-Alias find fd
Set-Alias ps procs
Set-Alias top btm
Set-Alias du dust

Invoke-Expression (& { (zoxide init powershell | Out-String) })
Invoke-Expression (& starship init powershell)
```

### `~/.config/bat/config`

```text
--theme="Monokai Extended"
--style="numbers,changes,header"
--paging=auto
```

---

## 6. Benchmarks récents (2026-05, x86_64 NVMe)

```
$ hyperfine 'grep -r "fn main" .' 'rg "fn main"'
Benchmark 1: grep -r "fn main" .
  Time (mean ± σ):     2.847 s ± 0.124 s
Benchmark 2: rg "fn main"
  Time (mean ± σ):     147.3 ms ±   8.9 ms
Summary
  'rg "fn main"' ran 19.32 ± 1.49 times faster than 'grep -r "fn main" .'

$ hyperfine 'find . -name "*.rs"' 'fd -e rs'
Benchmark 1: find . -name "*.rs"
  Time (mean ± σ):     1.293 s ± 0.041 s
Benchmark 2: fd -e rs
  Time (mean ± σ):     142.7 ms ±  11.2 ms
Summary
  'fd -e rs' ran 9.06 ± 0.78 times faster than 'find . -name "*.rs"'
```

---

## 7. Sources

- [Modern Rust CLI Tools (32blog)](https://32blog.com/en/cli/cli-modern-rust-tools)
- [5 Rust-Based CLI Tools (Medium)](https://medium.com/the-software-journal/5-rust-based-cli-tools-that-are-stupidly-fast-4cfcc9c5ce7c)
- [11 Rust CLI Tools 2026 (repotoire)](https://www.repotoire.com/blog/rust-cli-tools-2026)
- [Curated list (sts10/rust-command-line-utilities)](https://github.com/sts10/rust-command-line-utilities)
- [14 Rust-based Alternatives (itsfoss)](https://itsfoss.com/rust-alternative-cli-tools/)

---

## 8. Note Ubuntu 26.04 (cible #1 aphrody)

Ubuntu 25.10 a testé **uutils** (réimplémentation Rust de GNU coreutils) en
default. **Ubuntu 26.04 LTS** déploie uutils en production (cf. cible #1 aphrody
dans CLAUDE.md §0). Concrètement :

- `ls`, `cp`, `mv`, `cat`, `head`, `tail`, `cut`, `sort`, `uniq`, `wc`, etc. → binaires Rust
- `apt install rust-coreutils` (déjà default sur 26.04)
- Le toolbelt **modern alternatives** (rg/fd/sd/eza/bat) reste pertinent : uutils
  vise parité GNU, modern tools visent ergonomie/perf supérieure.

Pour aphrody, conséquence : **assume Rust coreutils sur Linux**, ne pas dépendre
de comportement GNU-exact (ex. `cp -r` flags non-standard).
