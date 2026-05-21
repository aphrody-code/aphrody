// SPDX-License-Identifier: Apache-2.0
//! `aphrody-llm-infra` — unified LLM runtime infrastructure.
//!
//! Merges four formerly standalone crates into a single cohesive library:
//!
//! - [`cost`] — per-provider USD cost + token usage tracker (pricing table,
//!   budget guard, NDJSON append-only log).
//! - [`rateguard`] — per-provider token-bucket rate limiter (RPM + TPM quotas,
//!   per-model overrides, async acquire).
//! - [`retry`] — reusable retry primitives (exponential backoff, jitter modes,
//!   classifier trait, `Retry-After` honoring).
//! - [`cache`] — two-tier cache: in-memory LRU with TTL + disk-backed
//!   persistent cache (atomic writes, SHA-256 keyed filenames).

#![forbid(unsafe_code)]

pub mod cost;
pub mod rateguard;
pub mod retry;
pub mod cache;
