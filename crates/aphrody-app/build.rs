// SPDX-License-Identifier: Apache-2.0
//! Tauri build hook: generates the capability schemas + embeds the config so
//! `tauri::generate_context!()` in `src/lib.rs` has everything it needs.

fn main() {
    tauri_build::build();
}
