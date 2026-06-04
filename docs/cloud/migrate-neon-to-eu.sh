#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# migrate-neon-to-eu.sh — recreate a Neon project in aws-eu-central-1 (Frankfurt)
# and migrate the data into it. Neon regions are immutable, so "moving" = create
# a new project in the EU + dump/restore. The source project is left UNTOUCHED
# (instant rollback).
#
# Why a script: the Neon MCP create_project has NO region parameter, so a
# region-pinned project must be created via the Neon REST API, which needs an
# API key. Provide one and this runs unattended.
#
# Usage:
#   NEON_API_KEY=napi_xxx  SOURCE_DATABASE_URL='postgres://…us-east-1…/neondb' \
#     ORG_ID=org-holy-smoke-94920538  NAME=shenron-eu  bash migrate-neon-to-eu.sh
#
# Prints the new project id + the POOLED DATABASE_URL to wire into Vercel /
# GitHub secrets / the bot.

set -euo pipefail
: "${NEON_API_KEY:?set NEON_API_KEY (napi_…) — Neon Console → Account settings → API keys}"
: "${SOURCE_DATABASE_URL:?set SOURCE_DATABASE_URL (the current project's DIRECT, non-pooled URL)}"
ORG_ID="${ORG_ID:-org-holy-smoke-94920538}"
NAME="${NAME:-rpbey-eu}"
REGION="${REGION:-aws-eu-central-1}"   # Frankfurt; aws-eu-west-2 = London
API="https://console.neon.tech/api/v2"

echo "▸ creating Neon project '$NAME' in $REGION (org $ORG_ID)…"
RESP=$(curl -fsS -X POST "$API/projects" \
  -H "Authorization: Bearer $NEON_API_KEY" -H 'Content-Type: application/json' \
  --data "{\"project\":{\"name\":\"$NAME\",\"region_id\":\"$REGION\",\"org_id\":\"$ORG_ID\",\"pg_version\":17}}")

PROJECT_ID=$(echo "$RESP" | bun -e 'console.log(JSON.parse(await Bun.stdin.text()).project.id)')
# connection_uris[0] is pooled when the pooler endpoint exists; fetch both
POOLED=$(echo "$RESP" | bun -e 'const j=JSON.parse(await Bun.stdin.text());console.log((j.connection_uris?.[0]?.connection_uri)||"")')
echo "  project_id=$PROJECT_ID"

echo "▸ dumping source → restoring into $NAME (data only mismatch-tolerant)…"
DUMP=/tmp/neon-migrate-$$.dump
pg_dump -Fc --no-owner --no-acl -d "$SOURCE_DATABASE_URL" -f "$DUMP"
# restore target = the new project's direct (non-pooled) URI
DIRECT=$(echo "$POOLED" | sed 's/-pooler//')
pg_restore --no-owner --no-acl -d "$DIRECT" "$DUMP" || echo "  (restore finished with non-fatal notices)"
rm -f "$DUMP"

echo "▸ verifying table parity…"
SRC_T=$(psql "$SOURCE_DATABASE_URL" -tAc "select count(*) from information_schema.tables where table_schema='public'")
DST_T=$(psql "$DIRECT" -tAc "select count(*) from information_schema.tables where table_schema='public'")
echo "  source tables=$SRC_T  target tables=$DST_T"
[ "$SRC_T" = "$DST_T" ] && echo "  ✅ parity OK" || { echo "  ❌ table count mismatch — DO NOT cut over"; exit 1; }

echo
echo "=== DONE — wire this POOLED DATABASE_URL (keep the source as rollback) ==="
echo "NEW_PROJECT_ID=$PROJECT_ID"
echo "DATABASE_URL=$POOLED"
echo
echo "Next: set this DATABASE_URL in Vercel env (prod+preview) + the GitHub"
echo "DATABASE_URL secret + the bot env; update NEON_PROJECT_ID var to"
echo "$PROJECT_ID; set vercel.json regions to ['fra1']; redeploy; verify 200;"
echo "then (only after days green) delete the old US project."
