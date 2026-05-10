//! Error module - User-friendly error classification system
//!
//! Provides structured error types with:
//! - Error codes for quick identification
//! - Clear messages and recovery hints
//! - Category classification for analytics
//!
//! Reference: OpenClaude src/utils/errors.ts, src/services/api/errors.ts

mod types;
mod classify;
mod api;
mod provider;

pub use types::{ErrorCode, ErrorCategory, QuickHorseError};
pub use classify::{classify_io_error, classify_json_error, classify_reqwest_error};
pub use api::{from_http_status, from_http_status_with_body};
pub use provider::{parse_api_error_body, classify_provider_error};

/// Result type alias using QuickHorseError
pub type Result<T> = std::result::Result<T, QuickHorseError>;

/// Convert any error to QuickHorseError
pub trait IntoQuickHorseError<T> {
    fn into_error(self) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> IntoQuickHorseError<T> for std::result::Result<T, E> {
    fn into_error(self) -> Result<T> {
        self.map_err(|e| QuickHorseError::unknown(e.to_string()))
    }
}

/// Helper macro for creating errors with context
#[macro_export]
macro_rules! err {
    // Error code only
    ($code:expr) => {
        QuickHorseError::new($code)
    };

    // Error code with message
    ($code:expr, $msg:expr) => {
        QuickHorseError::new($code).with_message($msg.to_string())
    };

    // Error code with message and details
    ($code:expr, $msg:expr, details: $details:expr) => {
        QuickHorseError::new($code)
            .with_message($msg.to_string())
            .with_details($details.to_string())
    };
}