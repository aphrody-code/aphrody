//! M3 motion tokens — easings + durations (M3 classic + Expressive).

/// M3 standard easing curves (cubic-bezier control points)
#[derive(Debug, Clone, Copy)]
pub struct Easing {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Easing {
    pub const EMPHASIZED:              Self = Self { x1: 0.2, y1: 0.0, x2: 0.0, y2: 1.0 };
    pub const EMPHASIZED_DECELERATE:   Self = Self { x1: 0.05, y1: 0.7, x2: 0.1, y2: 1.0 };
    pub const EMPHASIZED_ACCELERATE:   Self = Self { x1: 0.3, y1: 0.0, x2: 0.8, y2: 0.15 };
    pub const STANDARD:                Self = Self { x1: 0.2, y1: 0.0, x2: 0.0, y2: 1.0 };
    pub const STANDARD_DECELERATE:     Self = Self { x1: 0.0, y1: 0.0, x2: 0.0, y2: 1.0 };
    pub const STANDARD_ACCELERATE:     Self = Self { x1: 0.3, y1: 0.0, x2: 1.0, y2: 1.0 };
    pub const LINEAR:                  Self = Self { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 };
}

/// M3 duration tokens in milliseconds
pub mod duration {
    pub const SHORT1: u32 = 50;
    pub const SHORT2: u32 = 100;
    pub const SHORT3: u32 = 150;
    pub const SHORT4: u32 = 200;
    pub const MEDIUM1: u32 = 250;
    pub const MEDIUM2: u32 = 300;
    pub const MEDIUM3: u32 = 350;
    pub const MEDIUM4: u32 = 400;
    pub const LONG1: u32 = 450;
    pub const LONG2: u32 = 500;
    pub const LONG3: u32 = 550;
    pub const LONG4: u32 = 600;
    pub const EXTRA_LONG1: u32 = 700;
    pub const EXTRA_LONG2: u32 = 800;
    pub const EXTRA_LONG3: u32 = 900;
    pub const EXTRA_LONG4: u32 = 1000;
}
