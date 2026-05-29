// SPDX-License-Identifier: Apache-2.0
// ============================================================================
//  Bun-RS - Entry point re-exporting the Bun C-ABI dependencies
// ============================================================================

//! Library re-exports for the Bun Rust dependency graph.

/// Get the version of the wrapper crate.
pub fn wrapper_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
