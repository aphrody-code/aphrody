#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# MOVED: the yoyo sync autopilot now lives in the yoyo monorepo and is fully
# self-contained (resolves its own repo root + DB; no bxc coupling).
# Canonical: /home/ubuntu/yoyo/scripts/autopilot.sh
# The live crontab has been repointed there. This shim forwards for safety.
exec /home/ubuntu/yoyo/scripts/autopilot.sh "$@"
