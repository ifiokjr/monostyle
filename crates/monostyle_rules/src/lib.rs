//! The rule set.
//!
//! Every rule is a small function over a [`LexedFile`](monostyle_lexer::LexedFile) that
//! emits [`Finding`](monostyle_core::Finding)s. Rules never compute scores themselves;
//! they describe what they saw and let the scoring engine decide what it costs. That
//! separation is what guarantees every point lost has a rule name and an explanation
//! attached to it.
//!
//! Rules are grouped by the question they answer:
//!
//! - [`whitespace`] — is there room to breathe around control flow and between groups?
//! - [`structure`] — is the code flat, and are long parameter lists given space?
//! - [`comments`] — do comments explain why, and are they present where complexity
//!   demands them?
//! - [`complexity`] — are functions and files too complex to hold in your head?
//! - [`markdown`] — is the code inside documentation readable?

pub mod comments;
pub mod complexity;
pub mod config;
pub mod markdown;
pub mod metrics_bridge;
pub mod registry;
pub mod structure;
pub mod whitespace;

pub use config::RulesConfig;
pub use registry::all_rules;
pub use registry::run_rules;
