//! Profile-driven tokenizer.
//!
//! The lexer's job is to surface the signals that layout rules need without building a
//! full syntax tree. It produces a [`LexedFile`]: a line-oriented view of the source
//! where every line knows its indentation, whether it is blank or a comment, which
//! decisions it contains, and whether it opens parameters across multiple lines.
//!
//! Working line-first is what makes the readability rules practical. Blank-line
//! placement, grouping, and comment proximity are all one-dimensional questions about
//! a sequence of lines, so a line-oriented model answers them directly instead of
//! walking a tree and reconstructing layout from node spans.

pub mod comment;
pub mod lexer;
pub mod line;

pub use comment::CommentIntent;
pub use lexer::LexedFile;
pub use lexer::Unterminated;
pub use lexer::UnterminatedKind;
pub use lexer::lex;
pub use line::LexedLine;
pub use line::LineKind;
