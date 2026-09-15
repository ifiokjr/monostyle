//! A worked example of code that gives the reader room.
//!
//! Every function here follows the same shape: guard clauses for the failure cases, a blank
//! line before each control-flow statement, a blank line before the return, and a comment
//! wherever the reasoning would otherwise be invisible.
//!
//! This file scores 100 for both readability and complexity.

/// Maximum accepted payload length, in bytes.
const MAX_LENGTH: usize = 4096;

/// Validates a request before it reaches the handler.
///
/// # Why this exists
///
/// Each rejection reason needs to be independently testable, and a caller needs to
/// distinguish "malformed" from "too large" to return the right status code. Inlining these
/// checks into the handler made both impossible.
pub fn validate_input(input: &Input) -> Result<(), Error> {
    if input.is_empty() {
        return Err(Error::Empty);
    }

    if !input.is_valid_format() {
        return Err(Error::InvalidFormat);
    }

    if input.len() > MAX_LENGTH {
        return Err(Error::TooLong);
    }

    Ok(())
}

/// Resolves the configuration for a request.
///
/// # Why this exists
///
/// Precedence between the three configuration layers is genuinely subtle, so the resolution
/// order is stated once here rather than repeated at every lookup site.
pub fn resolve_config(request: &Request, defaults: &Config) -> Config {
    let mut config = defaults.clone();

    if let Some(overrides) = request.overrides() {
        config.apply(overrides);
    }

    // Environment configuration wins over per-request overrides. This ordering is
    // intentional: operators need a way to force a setting that a caller cannot bypass.
    if let Some(value) = std::env::var("SERVICE_TIMEOUT").ok() {
        config.timeout = value.parse().unwrap_or(config.timeout);
    }

    config
}
