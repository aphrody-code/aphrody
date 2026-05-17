# Growth Playbook — 30-day push toward 100,000 stars

This is the detailed companion to the `start` skill. `SKILL.md` gives the spine; this file gives the levers, templates, timing, and anti-patterns.

## The honest model of how stars happen

Stars come from one of three sources, in roughly this proportion during a launch month:

1. **Discovery from a launch post** (~70% of a launch-month's stars). Someone reads a Show HN, a top thread on /r/rust, a Lobste.rs front-page item, a viral tweet, a YouTube video. They click the repo, scan the README, scan the issues/PRs, and decide in ~30 seconds whether to star.
2. **Word-of-mouth from a respected node** (~25%). A well-followed engineer tweets, blogs, or namechecks the project. Their audience stars. This is what drives the 10k → 50k arc when it happens.
3. **Sustained organic flow** (~5% during the launch month, but ~100% long-term). SEO from blog posts, dependency listings, "awesome-X" inclusions. Slow and long-tail — not a 30-day lever.

In 30 days, the work is competing for (1) and (2). (3) is consequence, not strategy.

## The 30-second test

Open the repo in incognito as if you were a Rust/C engineer who has never seen it. Within 30 seconds, can the README answer:

- What does this *do*, in one line, no jargon?
- Why should I care, concretely — 10× faster than X, 10× less memory than Y, or does what nothing else does?
- Can I run it in 60 seconds from a clean machine?
- Is there a screenshot or gif that proves the claim?
- Does the code in the first linked file actually do what the README says?

If any answer is "no", that is the work. Promotion before the README passes the 30-second test wastes the launch.

## Channels ranked by 30-day star-yield

| Channel | Best for | Realistic outcome on a strong launch |
|---|---|---|
| Show HN (Tue/Wed 13–16 UTC, no marketing tone) | Infra, languages, OS, devtools, FFI work | 200–2000 stars on the day; sometimes 10× more |
| /r/rust top post | Rust-specific projects with technical depth | 100–800 stars |
| /r/cpp top post | C/C++ specific projects | 50–400 stars |
| Lobste.rs | Hard technical content; picky audience | 50–300 stars |
| Dev.to / Hashnode blog | SEO + cross-share fuel | 20–100 stars per strong post |
| Twitter/X technical thread | Only if you have a node or one retweets | Highly variable: 50 to 50k |
| YouTube demo video | Visual projects, devtools | Slow ramp; hundreds over weeks |
| "Awesome-X" PR inclusion | Established projects only | 50–200 stars over months |

## Show HN — title and body

The title decides 70% of whether the post breaks out. Bad titles get buried regardless of the post body.

**Title formula:** `Show HN: <name> — <specific testable technical claim>`

**Bad:** `Show HN: Google OS — Modern UNIX Kernel`
**Better:** `Show HN: Google OS — Zero-allocation FFI between Bun and C/Rust`
**Best:** `Show HN: Zero-alloc FFI lets Bun call C/Rust without GC pressure`

The good titles make a *specific testable claim*. The bad one is marketing.

**Body skeleton:**

```
Hi HN — I've been working on <name> for <duration>. It does <concrete one-line>.

The technical bit that makes it interesting: <2–4 sentences on the actual
mechanism — concrete, not "blazing fast". E.g., "We allocate the buffer in
JS once, pass the pointer down through bun:ffi, and the C side mutates in
place. No malloc/free on the hot path, no V8 GC touching the buffer.">

What's working today: <list 3 things, concrete>.
What's not yet: <list 2 things, honest>.

Code: <repo link>
Demo: <gif/video link>
Benchmark: <link to numbers if any>

Happy to answer questions — especially on <specific area where feedback is
genuinely useful>.
```

Don't say "revolutionary", "blazing fast", "production-ready" (unless it actually is and you can prove it). Engineers smell that copy.

## Reddit posts — title

`/r/rust`, `/r/cpp`, `/r/programming` reward: specific technical claim, slight self-deprecation, link to the *post* (not just the repo) where there's depth to read.

Bad: `Check out my new Rust kernel project!`
Good: `Replacing a C++ FFI bridge with Rust — what changed at the assembly level`

## Lobste.rs

Lobste.rs is small, picky, and high-quality. Submissions without depth get downvoted into the void. Submit *the technical post*, not the repo. Tag accurately (`rust`, `c`, `unix`, `practices`). Comment substantively on others' posts the week before — pure submitters get noticed and resented.

## Launch-day checklist (T-0)

Before the Show HN goes live:

- [ ] README has: one-line tagline, demo gif/video at top, 60-second install path, one paragraph on each of the three strongest claims, link to a technical post.
- [ ] CI badge is green (and actually verifying the workspace, not just one part).
- [ ] License badge present, license file is Apache 2.0 with proper headers in source.
- [ ] GitHub repo "About" is one technical sentence.
- [ ] GitHub topics set (`rust`, `c`, `ffi`, `bun`, `posix`, `windows`, etc.) — affects search discovery.
- [ ] Open issues triaged: nothing labeled `bug` left unaddressed at launch.
- [ ] PRs from before the launch are merged or closed with a reason.
- [ ] You can answer "how is this different from X?" in two sentences for the three closest competitors. Write these answers down — you will reuse them in comments.
- [ ] The technical post is published *before* the Show HN, not after — the post is the depth the curious reader clicks to.
- [ ] Tag the launch commit (e.g., `v0.1.0-launch`) so the repo state at launch is preserved.

## Post-launch (T+1 to T+7)

- Day 0 (launch): Answer every HN/Reddit comment within 2 hours during waking hours. Be technical, brief, humble on the gaps.
- Day 1–2: Triage every new issue. Label, acknowledge, time-box. People want to feel heard, and the responsiveness shows on the repo's pulse.
- Day 3–5: **Ship one user-requested feature within a week if at all reasonable.** Then post a short follow-up — "Heard from launch: top ask was X, here it is." This is the single highest-leverage post-launch action.
- Day 5–7: Do *not* re-launch on a new platform yet. Let the first wave land.

## When to cross-post

Wait 5–7 days after the initial launch before posting to the next big channel. Cross-posting too fast looks like a campaign and trips moderator flags.

A good order for this kind of project:
1. T+0: Show HN (highest discovery, hardest filter).
2. T+5: /r/rust *if the Rust angle is real and visible*. Link the technical post, not the repo.
3. T+7: Lobste.rs *if the depth justifies it*.
4. T+10: Dev.to mirror of the technical post (SEO compound interest).
5. T+14: YouTube demo (if there's a visual story).

## Anti-patterns — these cost stars and sometimes the project

- **README that overclaims.** First HN comment will say "actually this is just X with Y on top" and the thread dies. Underclaim, then exceed.
- **No demo gif/video.** Half the audience won't scroll the README without visual proof.
- **Many subreddit posts in a tight window.** Reads as spam → shadowban or removal.
- **Defensive replies to criticism.** Every "actually you're wrong because..." costs ~50 stars.
- **Buying stars / running star bots / nudging friends to mass-star.** GitHub's *inauthentic activity* detection unlists or deletes repos. The name becomes burned forever. Project-killer.
- **Posting Friday afternoon US time or weekends.** Engineering audiences are AFK. Tue–Thu 13–16 UTC wins for HN; mornings US for Reddit.
- **"We hit #1 on HN!!" victory laps before it actually hits #1.** Tone-deaf, costs goodwill, and HN's algorithm penalizes coordinated promotion.
- **Disabling issues or marking everything "wontfix" to keep the repo tidy.** New visitors interpret it as a dead or hostile project. Triage instead.

## When 100k is realistic and when it isn't

100k stars in 30 days has happened for: ChatGPT (the demo, after the announcement), a handful of generational AI tools at public release, GitHub Copilot-equivalent OSS launches. Bun's launch was extraordinary but took longer than 30 days to reach 100k.

For this project on this timeline, the most likely 30-day outcome with an excellent launch is **500–5000 stars** with a long tail building over months. The 100k target functions as moonshot framing — it forces every decision through "is this thing actually exceptional?" — and that framing is useful even when the literal number is unlikely.

**Diagnostic at D+21:** If the repo is at <500 stars, the answer is *more engineering value and a sharper angle*, not *more posting*. If the repo is at >5000 stars, double down on the angle that resonated and ship the most-requested feature.

## The single highest-leverage thing to do at any moment

When unsure what to ship next, ask: *what would make the next reader of this repo, who arrives knowing nothing, go "oh — that's interesting" in the first 30 seconds?* Ship that. Repeat.
