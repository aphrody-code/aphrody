<!-- SPDX-License-Identifier: Apache-2.0 -->

# Discord bot on Compute Engine `e2-micro` (free tier)

Source: Google Cloud blog "Build and run a Discord bot on Google Cloud" (uses a
Compute Engine VM running the gateway bot as a persistent process) · Bun guide
for the container · `gcloud` skill.

## Why a VM, not Cloud Run

rpbey's bot is a **gateway** bot with **voice/lavalink**:
- Gateway = a permanent WebSocket to Discord.
- Voice = UDP/RTP media.

Cloud Run is HTTP-only and CPU-throttles/scales idle instances to zero, which
drops the gateway and cannot carry voice UDP. A **Compute Engine VM** runs a real
always-on process with UDP — exactly what Google's own Discord-bot guide does.
The **`e2-micro` free tier** (1 VM/month in `us-west1`/`us-central1`/`us-east1`)
makes it ~$0.

> If you ever drop music/voice and use only slash commands, the bot could move to
> a Cloud Run service via the HTTP **Interactions** endpoint (ed25519-verified
> webhook) — but that is a different bot. As long as voice stays, use the VM.

## Provision the VM

```bash
gcloud compute instances create rpbey-bot \
  --project=rgfr-8927d --zone=us-west1-b \
  --machine-type=e2-micro \
  --image-family=debian-12 --image-project=debian-cloud \
  --boot-disk-size=30GB --boot-disk-type=pd-standard \
  --tags=rpbey-bot
```
(`e2-micro` + 30 GB `pd-standard` = within the Always-Free tier in those regions.)

## Run the bot as a Bun container (systemd)

SSH in (`gcloud compute ssh rpbey-bot --zone=us-west1-b`), install Docker (or
Bun directly), and run the bot container. Dockerfile (workspace-aware):

```docker
FROM oven/bun:latest
WORKDIR /app
COPY package.json bun.lock ./
COPY apps/bot apps/bot
COPY packages packages
RUN bun install --frozen-lockfile
WORKDIR /app/apps/bot
CMD ["bun", "src/index.ts"]
```

systemd unit `/etc/systemd/system/rpbey-bot.service`:
```ini
[Unit]
Description=rpbey Discord bot (Bun, gateway+voice)
After=network-online.target docker.service
Wants=network-online.target

[Service]
ExecStart=/usr/bin/docker run --rm --name rpbey-bot \
  --env-file /etc/rpbey/bot.env rpbey-bot:latest
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```
`sudo systemctl enable --now rpbey-bot`. (Or run Bun directly without Docker:
`ExecStart=/root/.bun/bin/bun /app/apps/bot/src/index.ts`.)

## Secrets (no tokens in code/image)

Use **Secret Manager** + the VM service account, or an `/etc/rpbey/bot.env`
(mode 600) holding `DISCORD_TOKEN`, `DATABASE_URL` (Neon pooled), lavalink creds.

```bash
echo -n "$DISCORD_TOKEN" | gcloud secrets create rpbey-discord-token --data-file=-
# grant the VM SA secretAccessor, fetch at boot into /etc/rpbey/bot.env
```

## Lavalink / voice

Lavalink needs its own always-on node (JVM). Options: run it as a second
container on the same `e2-micro` (tight on 1 GB RAM — watch memory), or a
separate small VM. Keep `@rpbey/lava-*` pointing at the node's host:port via env.

## Deploy automation

GitHub Action `deploy-bot.yml` (on push to `apps/bot/**`): build the image,
push to Artifact Registry, then `gcloud compute ssh ... 'docker pull && systemctl restart rpbey-bot'`
(or use a startup-script / OS Config). Auth via Workload Identity Federation
(no static SA key).

## Verify

```bash
gcloud compute ssh rpbey-bot --zone=us-west1-b --command='systemctl status rpbey-bot --no-pager'
gcloud compute ssh rpbey-bot --zone=us-west1-b --command='journalctl -u rpbey-bot -n 50 --no-pager'
```
Confirm the bot shows online in Discord and a slash command replies.
