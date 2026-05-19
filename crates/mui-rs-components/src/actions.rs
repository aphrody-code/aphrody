//! Actions: Button, IconButton, FAB, ButtonGroup.

/// M3 Button variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Filled,
    FilledTonal,
    Outlined,
    Elevated,
    Text,
}

pub struct Button {
    pub variant: ButtonVariant,
    pub label: String,
    pub disabled: bool,
    pub icon: Option<String>,
}
