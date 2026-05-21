//! Taffy layout engine integration — native CSS-like flexbox/grid.

pub use taffy::prelude::*;
use vello::kurbo::{Affine, Size as KurboSize};

/// Native style definition for M3 components, mapping to Taffy/CSS properties.
#[derive(Debug, Clone, Default)]
pub struct MuiStyle {
    pub style: Style,
}

impl MuiStyle {
    pub fn flex_row() -> Self {
        Self {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            }
        }
    }

    pub fn flex_column() -> Self {
        Self {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }
        }
    }

    pub fn grid(columns: Vec<Dimension>, rows: Vec<Dimension>) -> Self {
        Self {
            style: Style {
                display: Display::Grid,
                grid_template_columns: columns.into_iter().map(|d| minmax(d.into(), d.into())).collect(),
                grid_template_rows: rows.into_iter().map(|d| minmax(d.into(), d.into())).collect(),
                ..Default::default()
            }
        }
    }
}

pub struct LayoutEngine {
    pub taffy: TaffyTree<()>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    /// Computes the layout for the entire tree starting from the root.
    pub fn compute_layout(&mut self, root: NodeId, available_space: KurboSize) -> anyhow::Result<()> {
        let size = Size {
            width: AvailableSpace::Definite(available_space.width as f32),
            height: AvailableSpace::Definite(available_space.height as f32),
        };
        
        self.taffy.compute_layout(root, size).map_err(|e| anyhow::anyhow!("Layout error: {:?}", e))?;
        
        Ok(())
    }

    /// Gets the computed layout (position and size) for a specific node.
    pub fn get_layout(&self, node: NodeId) -> Option<&Layout> {
        self.taffy.layout(node).ok()
    }

    /// Helper to convert Taffy layout to Kurbo transform.
    pub fn get_transform(&self, node: NodeId) -> Affine {
        if let Some(layout) = self.get_layout(node) {
            Affine::translate((layout.location.x as f64, layout.location.y as f64))
        } else {
            Affine::IDENTITY
        }
    }
}
