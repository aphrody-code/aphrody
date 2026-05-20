//! Feedback: Badge, Snackbar, ProgressIndicator.

#[derive(Debug, Clone)]
pub struct Badge {
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Snackbar {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ProgressIndicator {
    pub progress: Option<f32>,
}
