<!-- SPDX-License-Identifier: Apache-2.0 -->

# Post-Launch Engagement Protocol (D+15 -> D+22)

Show HN is scheduled for D+15 (2026-06-01). The next 24h decides the run. This
document is the concrete playbook for the maintainer between submit and steady
state. Pair it with [`docs/COMPARISON.md`](COMPARISON.md) (honest diffs),
[`docs/ROADMAP.md`](ROADMAP.md), and [`docs/COMMUNITY.md`](COMMUNITY.md).

## 1. Pre-flight (T-1h, morning of launch)

- Verify repo public, README and code aligned, demo command runs end-to-end on
  Linux + Windows + WASM check, all CI green on `main`, latest commit < 24h old.
- Open browser tabs: HN submit page, Lobste.rs submit, `/r/rust` submit,
  Mastodon compose, Discord/Matrix invite link (when live per `COMMUNITY.md`).
- Have at hand: a 2-paragraph response to "Why not just use X?" for each
  competitor listed in [`docs/COMPARISON.md`](COMPARISON.md). Pre-write, do
  not improvise under load.
- Drain inbox, silence non-launch notifications, block 4 contiguous hours.

## 2. T+0 to T+1h (the critical first hour)

- Post the HN title agreed in the launch tracking issue.
- Stay at the keyboard. Reply within 10 minutes to every top-level comment.
- Do not defend. Acknowledge criticism, fix forward, link to commits.
- Watch the front-page ranking. HN algorithm decay is steep; first hour decides
  whether the post sees second-page traffic or goes dark.

## 3. T+1h to T+24h

- Cross-post to Lobste.rs after HN has stabilised, never in parallel; the
  pattern reads coordinated and both communities punish it.
- Cross-post to `/r/rust` after Lobste.rs, never before; reddit penalises
  duplicate-source submissions fast.
- Reply to all comments under 30 minutes.
- File a GitHub issue for every concrete bug raised even if the fix is 5
  minutes; the issue -> PR -> close cycle is publicly visible signal.
- Triage feature requests into [`docs/PLAN.md`](PLAN.md) under the wait status
  glyph, with the HN comment URL as source.

## 4. T+24h to T+72h

- Pin the top 3 most-asked questions as `FAQ.md` updates and issue templates.
- If launch landed (> 500 upvotes), write a follow-up post "Show HN +24h
  debrief" linking metrics and top engineering questions.
- Post the debrief URL on Mastodon and Discord. Skip Twitter unless an account
  is already warm.
- Acknowledge external contributors in `CHANGELOG.md` `Unreleased` section.

## 5. T+72h to T+7d

- Ship 1-2 most-requested feature PRs. Active maintainership is a stronger
  retention signal than any blog post.
- Reply in the HN thread once more if substantive comments arrive; do not
  necro-bump for engagement.
- Update [`docs/ROADMAP.md`](ROADMAP.md) based on launch feedback; mark
  reprioritised items with `(post-launch HN signal)`.
- Schedule the next post: "Aphrody at 1k stars: what we built", or the closest
  honest title to actual numbers.

## 6. T+7d (the steady-state pivot)

- Switch from launch ops to maintain ops.
- Set up `.github/dependabot.yml` notifications to email.
- Establish a weekly issue triage window (Monday 10:00 UTC suggested).
- Open Discord/Matrix per the `_pending_` items in
  [`docs/COMMUNITY.md`](COMMUNITY.md); do not open both until one has 50
  members or it splits the audience permanently.

## 7. Failure modes and recovery

- **HN flag-then-bury** within first hour: do not delete and repost; that path
  is a permanent ban. Email `hn@ycombinator.com` with the post URL and a short
  factual note requesting reinstatement. Success rate is low but non-zero.
- **Top comment is a serious bug**: reply within 30 min with "filed at #N, fix
  incoming", ship in < 24h, return with "fixed in commit X". Public closure
  beats public silence by a wide margin.
- **Top comment is "this is just X"**: reply with the differentiator from
  [`docs/COMPARISON.md`](COMPARISON.md). Do not defend in prose; respond with
  code references.
- **No traction (< 50 upvotes after 2h)**: do not repost same day. Wait 3 days,
  try a different title angle (never the same title twice).

## 8. Metrics to track

- HN upvotes, comment count, front-page minutes.
- GitHub stars hourly during launch day:
  `gh api repos/aphrody-code/aphrody --jq .stargazers_count`.
- Discord and Matrix join count (delta vs T-0).
- New issue and PR open rate (issues/hour, PRs/day).
- crates.io download spike on `aphrody` and `mrx`.

## 9. Mission alignment

Target: 1k stars within 7 days post-launch (D+22). Hitting 1k makes the 10k
Q3 target plausible. Coming in under 100 means the launch angle was wrong; do
not double down, re-evaluate the framing for the next post per `SHOW-HN.md`
candidate list.

## 10. After-action review

On D+22 write `docs/audits/2026-06-08-show-hn-aar.md` as an honest
retrospective: what worked, what did not, what to do differently next launch.
The AAR is owed to the project and to whoever runs the next launch (current
maintainer or otherwise).
