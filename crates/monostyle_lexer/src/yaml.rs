//! YAML block scalars keep embedded source text out of YAML's lexical signals.

/// Indentation owns a scalar after its mapping or sequence header closes.
pub(crate) struct BlockScalar {
	parent_indent: usize,
	content_indent: Option<usize>,
	started: bool,
}

/// An omitted indentation digit is inferred from the first nonblank payload line.
enum Indentation {
	Automatic,
	Explicit(usize),
}

impl BlockScalar {
	/// Reads an indicator only from unquoted, uncommented value position.
	pub(crate) fn from_header(raw: &str, masked: &str) -> Option<Self> {
		let value = scalar_value(masked)?;
		let indentation = indicator_indent(value)?;
		let parent_indent = mapping_indent(raw);

		let content_indent = match indentation {
			Indentation::Automatic => None,
			Indentation::Explicit(indent) => Some(parent_indent + indent),
		};

		Some(Self {
			parent_indent,
			content_indent,
			started: false,
		})
	}

	/// A blank line belongs to the scalar; nonblank content establishes its indent.
	pub(crate) fn contains_line(&mut self, characters: &[char]) -> bool {
		let mut content = characters
			.iter()
			.copied()
			.take_while(|c| !matches!(*c, '\r' | '\n'));
		let indent = content.clone().take_while(|c| *c == ' ').count();

		if content.all(char::is_whitespace) {
			return true;
		}

		if indent <= self.parent_indent {
			return false;
		}

		indent >= *self.content_indent.get_or_insert(indent)
	}

	/// Tells the scanner when to open the single contiguous protected body range.
	pub(crate) fn begin(&mut self) -> bool {
		!std::mem::replace(&mut self.started, true)
	}

	/// Pending headers do not yet own any payload characters.
	pub(crate) const fn started(&self) -> bool {
		self.started
	}
}

/// A mapping colon must separate its value; a bare sequence has no mapping key.
fn scalar_value(masked: &str) -> Option<&str> {
	if let Some((_, value)) = masked.split_once(':') {
		return value
			.starts_with(char::is_whitespace)
			.then_some(value.trim());
	}

	let mut value = masked.trim_start();
	let original = value.len();

	while let Some(next) = value.strip_prefix("- ") {
		value = next.trim_start();
	}

	(value.len() < original).then_some(value.trim())
}

/// Literal/folded indicators accept one chomping modifier and one indent digit.
fn indicator_indent(value: &str) -> Option<Indentation> {
	let mut characters = value.chars();

	if !matches!(characters.next(), Some('|' | '>')) {
		return None;
	}

	let digit = match (characters.next(), characters.next(), characters.next()) {
		(None | Some('+' | '-'), None, None) => return Some(Indentation::Automatic),
		(Some(digit @ '1'..='9'), None | Some('+' | '-'), None)
		| (Some('+' | '-'), Some(digit @ '1'..='9'), None) => digit,
		_ => return None,
	};

	digit
		.to_digit(10)
		.map(|width| Indentation::Explicit(width as usize))
}

/// Compact sequence mappings place their key after each dash and its whitespace.
fn mapping_indent(raw: &str) -> usize {
	let mut key = raw.trim_start_matches(' ');
	let mut indent = raw.len() - key.len();

	while let Some(after_dash) = key.strip_prefix("- ") {
		let next = after_dash.trim_start_matches(' ');

		if next.starts_with(['|', '>']) {
			break;
		}

		indent += key.len() - next.len();
		key = next;
	}

	indent
}
