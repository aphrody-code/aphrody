#!/usr/bin/env bash
# ============================================================================
#  aphrody — auto-deploy (bash, Linux/macOS)
#  Remplace l'ancien `cargo xtask deploy` (crate aphrody-xtask supprimée le
#  2026-05-21). Build release de tous les binaires du workspace puis copie
#  ceux qui matchent les préfixes vers ~/.local/bin (convention d'install).
#
#  Usage :
#    ./scripts/deploy.sh                        # build + install (aphrody,mrx,notebooklm)
#    ./scripts/deploy.sh --no-build             # copie seulement (build déjà fait)
#    ./scripts/deploy.sh --prefixes aphrody     # restreindre aux bins aphrody*
#    ./scripts/deploy.sh --target x86_64-unknown-linux-gnu
#    ./scripts/deploy.sh --dest /opt/bin --dry-run
#    ./scripts/deploy.sh --include-gui          # + copie aphrody-gui (app desktop)
#  Parité : scripts/deploy.ps1 (Windows).
#
#  Note GUI : l'app desktop (apps/desktop) est un workspace SELF-ROOTED exclu du
#  workspace core, donc `cargo build --release` ne la construit PAS. --include-gui
#  copie le binaire `aphrody-gui` déjà bâti (copy-only) depuis
#  apps/desktop/src-tauri/target/{release,debug} à côté d'aphrody, pour que le
#  resolver sibling de `aphrody gui` le trouve une fois déployé. Construire la
#  GUI d'abord : cd apps/desktop && bun install && bun run build && cd src-tauri
#  && cargo build --release.
# ============================================================================
set -euo pipefail

NO_BUILD=0
PREFIXES="aphrody,mrx,notebooklm"
DEST=""
TARGET=""
DRY_RUN=0
INCLUDE_GUI=0

while [ $# -gt 0 ]; do
    case "$1" in
        --no-build)    NO_BUILD=1; shift ;;
        --prefixes)    PREFIXES="$2"; shift 2 ;;
        --dest)        DEST="$2"; shift 2 ;;
        --target)      TARGET="$2"; shift 2 ;;
        --include-gui) INCLUDE_GUI=1; shift ;;
        --dry-run)     DRY_RUN=1; shift ;;
        -h|--help)     sed -n '2,28p' "$0"; exit 0 ;;
        *) echo "Argument inconnu : $1" >&2; exit 2 ;;
    esac
done

# --- Racine repo (le script vit dans scripts/) ------------------------------
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# --- Préfixes ---------------------------------------------------------------
IFS=',' read -r -a PREFIX_ARR <<< "$PREFIXES"
[ "${#PREFIX_ARR[@]}" -gt 0 ] || { echo "--prefixes vide" >&2; exit 2; }

# --- Destination ------------------------------------------------------------
if [ -z "$DEST" ]; then
    DEST="${HOME:-.}/.local/bin"
fi

# --- Build ------------------------------------------------------------------
if [ "$NO_BUILD" -eq 0 ]; then
    BUILD_ARGS=(build --release --locked)
    [ -n "$TARGET" ] && BUILD_ARGS+=(--target "$TARGET")
    echo "[deploy] cargo ${BUILD_ARGS[*]}"
    if [ "$DRY_RUN" -eq 0 ]; then
        cargo "${BUILD_ARGS[@]}"
    fi
fi

# --- Répertoire(s) des artefacts -------------------------------------------
# Sans --target explicite, `.cargo/config.toml` peut forcer un `build.target`
# : les binaires atterrissent alors dans target/<triple>/release et non
# target/release. On cherche dans tous les candidats plausibles.
matches_prefix() {
    local name="$1"
    for p in "${PREFIX_ARR[@]}"; do
        case "$name" in "$p"*) return 0 ;; esac
    done
    return 1
}

CANDIDATE_DIRS=()
if [ -n "$TARGET" ]; then
    CANDIDATE_DIRS+=("$REPO_ROOT/target/$TARGET/release")
else
    CANDIDATE_DIRS+=("$REPO_ROOT/target/release")
    for d in "$REPO_ROOT"/target/*/release; do
        [ -d "$d" ] && CANDIDATE_DIRS+=("$d")
    done
fi

# --- Découverte : exécutables top-level matchant un préfixe -----------------
BINS=()
RELEASE_DIR=""
for dir in "${CANDIDATE_DIRS[@]}"; do
    [ -d "$dir" ] || continue
    mapfile -t found < <(
        find "$dir" -maxdepth 1 -type f -perm -u+x ! -name '*.d' ! -name '*.so' 2>/dev/null \
            | while read -r f; do
                base="$(basename "$f")"
                if matches_prefix "$base"; then echo "$f"; fi
            done | sort -u
    )
    if [ "${#found[@]}" -gt 0 ]; then BINS=("${found[@]}"); RELEASE_DIR="$dir"; break; fi
done
[ "${#BINS[@]}" -gt 0 ] || { echo "Aucun binaire exécutable matchant $PREFIXES dans : ${CANDIDATE_DIRS[*]} (build manqué ?)" >&2; exit 1; }
echo "[deploy] artefacts depuis $RELEASE_DIR"

# --- GUI desktop (opt-in) : binaire hors-workspace, build séparé ------------
# `aphrody-gui` est produit par le cargo self-rooted de apps/desktop/src-tauri
# (target propre), jamais par le build core. On l'ajoute au lot de copie pour
# qu'il atterrisse à côté d'aphrody (le resolver sibling de `aphrody gui` le
# trouve alors en prod). Copy-only : aucun build déclenché ici.
if [ "$INCLUDE_GUI" -eq 1 ]; then
    GUI_DIRS=()
    if [ -n "$TARGET" ]; then
        GUI_DIRS+=("$REPO_ROOT/apps/desktop/src-tauri/target/$TARGET/release")
    else
        GUI_DIRS+=("$REPO_ROOT/apps/desktop/src-tauri/target/release" \
                   "$REPO_ROOT/apps/desktop/src-tauri/target/debug")
    fi
    gui_bin=""
    for gd in "${GUI_DIRS[@]}"; do
        for name in aphrody-gui aphrody-gui.exe; do
            if [ -f "$gd/$name" ]; then gui_bin="$gd/$name"; break 2; fi
        done
    done
    if [ -n "$gui_bin" ]; then
        BINS+=("$gui_bin")
        echo "[deploy] +GUI : $gui_bin"
    else
        echo "[warn] --include-gui demandé mais aphrody-gui introuvable dans : ${GUI_DIRS[*]}" >&2
        echo "       Build : cd apps/desktop && bun install && bun run build && cd src-tauri && cargo build --release" >&2
    fi
fi

[ "$DRY_RUN" -eq 0 ] && mkdir -p "$DEST"

# --- Copie ------------------------------------------------------------------
deployed=0
for src in "${BINS[@]}"; do
    base="$(basename "$src")"
    dst="$DEST/$base"
    if [ "$DRY_RUN" -eq 1 ]; then
        echo "[dry-run] $src -> $dst"
        continue
    fi
    # install(1) gère le remplacement même si le binaire tourne (unlink+create).
    if install -m 0755 "$src" "$dst" 2>/dev/null || cp -f "$src" "$dst"; then
        size=$(stat -c%s "$dst" 2>/dev/null || stat -f%z "$dst" 2>/dev/null || echo "?")
        printf "  [ok] %-28s (%s bytes)\n" "$base" "$size"
        deployed=$((deployed+1))
    else
        echo "  [fail] $base" >&2
    fi
done

[ "$DRY_RUN" -eq 0 ] && echo "=== aphrody deploy : $deployed binaire(s) installé(s) -> $DEST ==="

# --- Check PATH -------------------------------------------------------------
case ":$PATH:" in
    *":$DEST:"*) : ;;
    *) echo "[warn] $DEST n'est pas dans le PATH. Ajoute :  export PATH=\"\$PATH:$DEST\"" ;;
esac
