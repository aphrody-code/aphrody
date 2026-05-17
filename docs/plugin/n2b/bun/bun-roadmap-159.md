# Bun's Roadmap

_Statut : OPEN · créé 2022-05-06 · mis à jour 2026-01-02_

Source : https://github.com/oven-sh/bun/issues/159

---

# Bun's Roadmap

The Bun team updates this issue to share our current roadmap, priorities, and goals. You can see our current update below, and previous updates at the bottom.

## Spring 2025

### Node.js compatibility

For every change we make to Bun, we run the Node.js test suite to ensure that Bun is compatible with Node.js. We also started a [bounty program](https://x.com/jarredsumner/status/1914830430811177181) within our team, to accelerate our progress.

<p align="center">
<img width="591" alt="Image" src="https://github.com/user-attachments/assets/e6efa3b3-6aa7-4d84-8d71-ac4556c3b1ca" />
</p>

Our goal is to be 90% compatible with Node.js. We'll be providing [updates](https://x.com/bunjavascript) as our progress nears completion.

### Features

While we continue to work on Node.js compatibility and bug fixes, we’re also going to be working on new features in upcoming releases of Bun, including:

- [x] [`#18812`](https://github.com/oven-sh/bun/pull/18812) [Redis](https://bun.sh/docs/api/redis) client
- [x] [`#19699`](https://github.com/oven-sh/bun/issues/19699) MySQL support with [`Bun.sql`](https://bun.sh/docs/api/sql)
- [x] [`#19701`](https://github.com/oven-sh/bun/issues/19701) SQLite support with `Bun.sql` (in addition to the existing [`bun:sqlite`](https://bun.sh/docs/api/sqlite) API)
- [x] [`#4824`](https://github.com/oven-sh/bun/issues/4824) Test runner support in VSCode
- [ ] [`#7589`](https://github.com/oven-sh/bun/issues/7589) Running scripts with filters, and in parallel, using `bun run`
- [ ] [`#947`](https://github.com/oven-sh/bun/issues/947) REPL support (replacement for existing, 3rd-party package: `bun repl`)
- [ ] Improve usage of `bun install` at work, including:
  - [x] [`#7157`](https://github.com/oven-sh/bun/issues/7157) Support migration from `pnpm-lock.yaml` to `bun.lock`
  - [x] [`#4844`](https://github.com/oven-sh/bun/issues/4844) Support [`catalogs`](https://pnpm.io/catalogs)
  - [ ] [`#6608`](https://github.com/oven-sh/bun/issues/6608) Support nested `resolutions` and `overrides`
  - [ ] Fix issues with workspaces, private registries, and corporate proxies

### Reliability

We're also hard at work improving Bun's reliability, for example, reducing crashes and memory leaks. We've been working on a few projects to help with this.

- [x] [`#19057`](https://github.com/oven-sh/bun/pull/19057) Address sanitizer (aka. ASAN)

    We now run address sanitizer in our CI, to detect bugs like heap buffer overflows

- [x] [`v1.2.2`](https://bun.sh/blog/bun-v1.2.2#javascript-uses-10-30-less-memory-at-idle) 10-30% less memory usage when idle

    We made changes to how Bun schedules garbage collection, so that it syncs with JavaScriptCore. This improves memory usage when Bun is idle.

- [ ] Distribute a debug/assertion build of Bun

    Sometimes it's difficult to reproduce crashes without having access to your source code. We will soon distribute a special, optional build of Bun that includes more assertions and debug logs, so that it's easier for you to provide us with reproductions of issues.

### Hiring

Bun’s team is growing!

- 3 years ago, it was just Jarred.
- 2 years ago, we raised a Seed round and grew to a team of 4.
- Today, we're 14 people

We’re also announcing new job openings at our San Francisco office, including:

- [Head of Propaganda](https://apply.workable.com/bun/j/FAD66BC1D1/) (aka. Developer Advocate) for $120k-$180k + Equity
- [Engineering Manager](https://apply.workable.com/bun/j/A3EC729D81/) for $180k-$240k + Equity
- [Senior Systems Engineer(s)](https://apply.workable.com/bun/j/A7A1388873/) for $160k-$220k + Equity

If you’re excited about working at Bun, you can apply at [bun.sh/careers](https://bun.sh/careers).

### Something new

Stay tuned.

<p align="center">
<img width="598" alt="Image" src="https://github.com/user-attachments/assets/b40f2da3-906d-4681-a40a-3cea04303b35" />
</p>

## Previous updates

<details>

<summary><strong>Winter 2024</strong></summary>

### bun install

- [x] https://github.com/oven-sh/bun/issues/11863 (will unblock https://github.com/dependabot/dependabot-core/issues/6528) 
- [ ] https://github.com/oven-sh/bun/issues/6608
- [ ] Fixes for frequently-reported issues for things like private git URLs and some edgecases with hoisting
- [ ] `bun update --interactive` (https://github.com/oven-sh/bun/issues/4895)
- [x] https://github.com/oven-sh/bun/issues/271
- [ ] https://github.com/oven-sh/bun/issues/5846
- [ ] Update `bun init` to add `"engines": { "bun": ...` by default. If `bun` present then ignore node shebangs by default ([#9346](https://github.com/oven-sh/bun/issues/9346))
- [x] https://github.com/oven-sh/bun/issues/692

### Runtime

#### Node.js compatibility

- [x] Implement `node:http2` server to unblock grpc2
- [ ] Implement more of V8 C++ APIs to unblock canvas, node-pty
- [ ] Rewrite node:http (https://github.com/oven-sh/bun/pull/14384)
- [ ] Add much more comprehensive test coverage for napi
- [ ] Get at least 75% of Node's test suite running on every commit (as of last update: 37% currently). This number will go up in future  
- [ ] Investigate removing our `undici` override
- [x] https://github.com/oven-sh/bun/issues/13681
- [x] https://github.com/oven-sh/bun/issues/1723

#### Reliability

- [ ] https://github.com/oven-sh/bun/issues/15141 

### Bake (Bundler)

The goal for Bake is to make Bun the most productive tool for building static & full-stack JavaScript and TypeScript applications, leveraging runtime, bundler, and transpiler integration to make things simpler.

High level:
- [ ] **Make a fast HMR development full-stack server** - https://github.com/oven-sh/bun/issues/14324
- [ ] Production builds (#14763)
    - [x] Static production builds
- [ ] Implement an integration with a popular framework (such as Next.js)
- [ ] Easy & powerful plugin API

### bun test

- [x] Reporter API or socket API

### Organizational

- [x] Hire a contractor to help us with CI
- [ ] Hire a technical writer to help with docs
- [ ] Hire an engineering-focused role for maintaining the TypeScript types & frontend for docs +  help with integrations with third-party packages
- [ ] Hire more systems engineers

</details>

<details> 

<summary><strong>Fall 2024</strong></summary>

### bun install

Essentially, feature complete.
- [x] `bun outdated`
- [x]  https://github.com/oven-sh/bun/issues/487
- [x] `bun publish`
- [ ] Text-based lockfile format (which will help unblock https://github.com/dependabot/dependabot-core/issues/6528) 
- [ ] `bun update --interactive` (https://github.com/oven-sh/bun/issues/4895)
- [ ] Fixes for frequently-reported issues for things like private git URLs and some edgecases with hoisting

### Runtime

#### Node.js compatibility

- [x] TextEncoderStream & TextDecoderStream 
- [x] V8 C++ API (in-progress)
- [x] Implement `node:cluster`
- [ ] Implement `node:http2` server to unblock grpc2
- [x] Rewrite `node:zlib` to address performance issues in some common packages
- [x] https://github.com/oven-sh/bun/issues/13681
- [ ] Fix various bugs in `node:http`, avoid wrapping Bun.serve() and fetch() and use a more direct implementation
- [ ] Add much more comprehensive test coverage for napi
- [ ] Get at least 25% of Node's test suite running on every commit (as of last update: 15% currently). This number will go up in future quarters.

#### Reliability

- [ ] Delete almost all code that manually reads JSValue. Replace with an IDL bindings generator for JavaScriptCore objects/classes that supports C++ & Zig output, and importantly: function arguments, return values, and exceptions. It should make the lifetime of these values brainless for us to reason about. 
- [x] Address TLS-related issues that have cropped up
- [x] Continue to allocate significant time to fixing bugs and improve test coverage as they crop up 

### Bundler

- [x] Fix many common bugs people run into related to source maps or incorrect output
- [ ] A new, higher-level iteration of Bun's bundler designed for server-driven JavaScript #14324
- [x] CSS parser and bundler #14167

### bun test

- [ ] Reporter API or socket API

### Organizational

- [ ] Hire a contractor to help us with CI
- [ ] Hire a technical writer to help with docs
- [ ] Hire an engineering-focused role for maintaining the TypeScript types & frontend for docs +  help with integrations with third-party packages
- [ ] Hire more systems engineers

</details>
