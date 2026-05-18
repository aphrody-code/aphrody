// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 component implementations.
//!
//! Each component is a plain Rust struct implementing
//! [`crate::MaterialComponent`].  Components own their interaction state
//! (`hover`, `pressed`, `focused`, …) and emit geometry into a
//! [`crate::Canvas`] when [`crate::MaterialComponent::paint`] is called.
//!
//! All component metrics are sourced from the canonical M3 component
//! specification (heights, radii, padding, type style).

pub mod button;
pub mod card;
pub mod dialog;
pub mod fab;
pub mod navigation_bar;
pub mod slider;
pub mod snackbar;
pub mod switch;
pub mod tabs;
pub mod text_field;

pub use button::{Button, ButtonVariant};
pub use card::{Card, CardVariant};
pub use dialog::Dialog;
pub use fab::{Fab, FabSize};
pub use navigation_bar::{NavigationBar, NavigationDestination};
pub use slider::Slider;
pub use snackbar::Snackbar;
pub use switch::Switch;
pub use tabs::{Tab, Tabs, TabsVariant};
pub use text_field::{TextField, TextFieldVariant};
