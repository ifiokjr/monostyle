//! Language profiles.
//!
//! A [`LanguageProfile`] is the data that makes one lexer work for every language.
//! Rather than shipping a parser per language, monostyle describes each language as data:
//! how comments start, which delimiters form strings, whether block comments nest, how
//! interpolation is written, and which keywords create decision points.
//!
//! Readability is a property of layout, not of grammar. A profile-driven lexer sees exactly
//! what the layout rules need — blank lines, indentation, comment placement, and branch
//! points — while staying fast and uniform across every supported language. It also means
//! adding a language is a data edit rather than a code change.

pub mod catalog;
pub mod profile;

pub use catalog::profile_for;
pub use profile::BlockStyle;
pub use profile::HeredocSyntax;
pub use profile::Interpolation;
pub use profile::LanguageProfile;
pub use profile::StringRule;
