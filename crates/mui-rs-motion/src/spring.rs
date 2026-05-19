//! M3 spring physics — semi-implicit Euler integrator.
//! M3 Expressive spring params: stiffness, damping, mass.
//! ~150 LOC, no rapier2d dependency.

/// Spring parameters (M3 Expressive spec)
#[derive(Debug, Clone, Copy)]
pub struct SpringParams {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl SpringParams {
    /// Standard M3 spring — used for most transitions
    pub const STANDARD: Self = Self { stiffness: 380.0, damping: 40.0, mass: 1.0 };
    /// Emphasized spring — bouncy for FAB, dialogs
    pub const EMPHASIZED: Self = Self { stiffness: 300.0, damping: 30.0, mass: 1.0 };
    /// Decelerate — entering elements
    pub const DECELERATE: Self = Self { stiffness: 600.0, damping: 55.0, mass: 1.0 };
    /// Accelerate — exiting elements
    pub const ACCELERATE: Self = Self { stiffness: 600.0, damping: 45.0, mass: 1.0 };
}

/// Stateful spring integrator for a single scalar value.
#[derive(Debug, Clone)]
pub struct Spring {
    params: SpringParams,
    position: f32,
    velocity: f32,
    target: f32,
}

impl Spring {
    pub fn new(params: SpringParams, initial: f32) -> Self {
        Self { params, position: initial, velocity: 0.0, target: initial }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Advance by `dt` seconds (semi-implicit Euler).
    pub fn tick(&mut self, dt: f32) -> f32 {
        let SpringParams { stiffness, damping, mass } = self.params;
        let displacement = self.position - self.target;
        let spring_force = -stiffness * displacement;
        let damping_force = -damping * self.velocity;
        let acceleration = (spring_force + damping_force) / mass;
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;
        self.position
    }

    pub fn is_at_rest(&self, tolerance: f32) -> bool {
        (self.position - self.target).abs() < tolerance && self.velocity.abs() < tolerance
    }

    pub fn position(&self) -> f32 {
        self.position
    }
}
