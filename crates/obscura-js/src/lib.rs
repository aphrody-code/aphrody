#![allow(unused_imports)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::let_underscore_future)]
#![allow(clippy::new_without_default)]
#![allow(clippy::redundant_closure)]

extern crate html5ever;

pub mod module_loader;
pub mod runtime;
pub mod ops;
pub mod v8_flags;
pub mod v8_lock;
pub mod markdown;

pub use markdown::HTML_TO_MARKDOWN_JS;
pub use v8_flags::set_v8_flags;
