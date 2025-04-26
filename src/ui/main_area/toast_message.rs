use egui::Color32;
use std::time::Instant;

/// Toast notification message
#[derive(Clone)]
pub struct ToastMessage {
    pub message: String,
    pub expires_at: Instant,
    pub color: Color32,
}

impl ToastMessage {
    /// Create a new toast message
    pub fn new(message: String, color: Color32, duration_secs: u64) -> Self {
        Self {
            message,
            expires_at: Instant::now() + std::time::Duration::from_secs(duration_secs),
            color,
        }
    }
    
    /// Check if the toast message has expired
    pub fn has_expired(&self) -> bool {
        self.expires_at <= Instant::now()
    }
}
