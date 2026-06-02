#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# vps-google-scrape.sh — Scrapes Google Search results via bxc and saves them.

set -e

# Load user profile path for Chromium
export HOME="${HOME:-$(getent passwd "$(id -un)" | cut -d: -f6 || echo "/home/ubuntu")}"
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$HOME/.bun/bin:$HOME/.local/bin"

DATA_DIR="$HOME/data/google-scraped"
mkdir -p "$DATA_DIR"

QUERIES=(
  "artificial+intelligence"
  "rust+programming"
  "webassembly+development"
  "multi+agent+systems"
  "google+design+3"
)

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

echo "=== [${TIMESTAMP}] Starting Google scraping job via bxc ==="

for Q in "${QUERIES[@]}"; do
  CLEAN_Q=$(echo "$Q" | tr -d '+' | tr -d ' ' | tr -cd '[:alnum:]')
  OUT_FILE="${DATA_DIR}/${TIMESTAMP}_${CLEAN_Q}.json"
  URL="https://www.google.com/search?q=${Q}&udm=14"
  
  echo "Scraping: ${URL} -> ${OUT_FILE}"
  
  # Run bxc recon with static profile and output json
  if bxc recon "${URL}" --profile static --json > "${OUT_FILE}.tmp" 2>/dev/null; then
    mv "${OUT_FILE}.tmp" "${OUT_FILE}"
    echo "Successfully scraped ${Q}."
  else
    rm -f "${OUT_FILE}.tmp"
    echo "ERROR: Failed to scrape ${Q}."
  fi
done

# Keep only the last 50 files to avoid filling up the disk
find "$DATA_DIR" -name "*.json" -type f -mtime +7 -delete

echo "=== Google scraping job completed ==="
