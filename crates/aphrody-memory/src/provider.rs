// SPDX-License-Identifier: Apache-2.0
//! Tier-1 [`MemoryProvider`] trait — dyn-compatible async surface.
//!
//! Three implementations ship in this crate:
//!
//! - [`crate::mem0::Mem0Provider`] — hosted Mem0 cloud (HTTP).
//! - [`crate::honcho::HonchoProvider`] — hosted Honcho v1 API (HTTP).
//! - [`crate::sqlite_local::SqliteLocalProvider`] — offline rusqlite store.
//!
//! Callers typically erase to `Box<dyn MemoryProvider>` so the active backend
//! can be swapped via [`crate::types::MemoryConfig`] without touching call
//! sites. The trait is intentionally narrow (add / get / search / delete /
//! health / kind) so every backend can implement it without contortions.

use async_trait::async_trait;

use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind};

/// Long-term agent memory backend, swappable at runtime.
///
/// Dyn-compatible: `Box<dyn MemoryProvider>` and `&dyn MemoryProvider` are
/// both legal. All implementations are `Send + Sync` so they survive being
/// shared across tokio tasks.
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// Persist `rec` and return the canonical id.
    ///
    /// HTTP providers ignore `rec.id` and adopt the server-assigned id.
    /// [`crate::sqlite_local::SqliteLocalProvider`] generates a UUID v4 if
    /// `rec.id` is empty.
    async fn add(&self, rec: MemoryRecord) -> Result<String, MemoryError>;

    /// Fetch one record by id. `Ok(None)` means *no such id*; an actual
    /// transport failure surfaces as [`MemoryError::Http`] or
    /// [`MemoryError::ApiError`].
    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError>;

    /// Search records matching `q` (see [`MemoryQuery`] for filter semantics).
    /// Results are unordered unless the backend documents otherwise.
    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError>;

    /// Remove a record by id. Idempotent: deleting an absent id returns `Ok(())`.
    async fn delete(&self, id: &str) -> Result<(), MemoryError>;

    /// Liveness check — issues a cheap read-only call to the provider so
    /// callers can fail fast on misconfigured credentials.
    async fn health(&self) -> Result<(), MemoryError>;

    /// Discriminator for logs, metrics, and conditional fallback.
    fn provider_kind(&self) -> ProviderKind;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time proof: `dyn MemoryProvider` is object-safe. The fn body
    /// is never called; the existence of the type alias is the assertion.
    #[allow(dead_code)]
    fn dyn_compat_check(_: Box<dyn MemoryProvider>) {}
}
