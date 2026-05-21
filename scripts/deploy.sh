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
#  Parité : scripts/deploy.ps1 (Windows).
# ============================================================================
set -euo pipefail

NO_BUILD=0
PREFIXES="aphrody,mrx,notebooklm"
DEST=""
TARGET=""
DRY_RUN=0

while [ $# -gt 0 ]; do
    case "$1" in
        --no-build)  NO_BUILD=1; shift ;;
        --prefixes)  PREFIXES="$2"; shift 2 ;;
        --dest)      DEST="$2"; shift 2 ;;
        --target)    TARGET="$2"; shift 2 ;;
        --dry-run)   DRY_RUN=1; shift ;;
        -h|--help)   sed -n '2,20p' "$0"; exit 0 ;;
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

# --- Répertoire des artefacts ----------------------------------------------
if [ -n "$TARGET" ]; then
    RELEASE_DIR="$REPO_ROOT/target/$TARGET/release"
else
    RELEASE_DIR="$REPO_ROOT/target/release"
fi
[ -d "$RELEASE_DIR" ] || { echo "Répertoire release introuvable : $RELEASE_DIR (build manqué ?)" >&2; exit 1; }

# --- Découverte : exécutables top-level matchant un préfixe -----------------
matches_prefix() {
    local name="$1"
    for p in "${PREFIX_ARR[@]}"; do
        case "$name" in "$p"*) return 0 ;; esac
    done
    return 1
}

mapfile -t BINS < <(
    find "$RELEASE_DIR" -maxdepth 1 -type f -perm -u+x ! -name '*.d' ! -name '*.so' 2>/dev/null \
        | while read -r f; do
            base="$(basename "$f")"
            if matches_prefix "$base"; then echo "$f"; fi
        done | sort -u
)
[ "${#BINS[@]}" -gt 0 ] || { echo "Aucun binaire exécutable matchant $PREFIXES dans $RELEASE_DIR" >&2; exit 1; }

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
