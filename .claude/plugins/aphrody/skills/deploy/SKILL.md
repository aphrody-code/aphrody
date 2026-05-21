---
name: deploy
description: "Deploy application to production. TRIGGER when: user says 'deploy', 'ship', 'push to prod', 'release', 'mise en prod', or asks to restart a service. Handles build verification, systemd services, and deployment validation."
allowed-tools: Read, Bash, Glob, Grep
model: inherit
user-invocable: true
version: "1.0"
---

# Deploy to Production

Launch the `deployment-engineer` agent to handle the full deployment pipeline.

## Pre-flight Context

Git status: !`git status --short 2>/dev/null`
Current branch: !`git branch --show-current 2>/dev/null`
Uncommitted changes: !`git diff --stat 2>/dev/null | tail -1`

## Instructions

1. Launch the `deployment-engineer` agent
2. Pass it the deployment target based on user request:
   - a specific service / binary name → that target only
   - "all" or unspecified → the full release set
3. The deployment-engineer will handle: pre-flight → build → deploy → validate

$ARGUMENTS
