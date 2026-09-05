// SPDX-License-Identifier: Apache-2.0
//! Emits the materialised three-way UI fusion stylesheet (Material 3 + shadcn
//! + Tailwind v4) to stdout: M3 `--md-sys-color-*` system colors, the shadcn
//! semantic-variable alias block, and the Tailwind `@theme inline` block.
//!
//! Run: `cargo run -p m3-tokens --example fusion > tokens.css`
//! See `docs/design/FUSION-PLAN.md`.

fn main() {
    use m3_tokens::color::{BASELINE, export_fusion_css};
    print!("{}", export_fusion_css(&BASELINE));
}
