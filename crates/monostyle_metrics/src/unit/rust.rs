//! Rust declarations have visibility parentheses before their parameter lists.

/// Finds a Rust function declaration without treating visibility or data types as functions.
pub(super) fn function_name(code: &str) -> Option<String> {
	const QUALIFIERS: &[&str] = &["async", "const", "default", "extern", "safe", "unsafe"];

	let mut declaration = without_visibility(code.trim())?;

	loop {
		let (keyword, rest) = declaration.split_once(char::is_whitespace)?;

		if keyword == "fn" {
			return identifier_before_parameters(rest.trim_start());
		}

		if !QUALIFIERS.contains(&keyword) {
			return None;
		}

		declaration = rest.trim_start();
	}
}

/// Removes an optional visibility prefix; its parentheses never name a callable.
fn without_visibility(code: &str) -> Option<&str> {
	let Some(rest) = code.strip_prefix("pub") else {
		return Some(code);
	};
	let boundary = rest.chars().next()?;

	if boundary != '(' && !boundary.is_whitespace() {
		return None;
	}

	let rest = rest.trim_start();

	if let Some(scope) = rest.strip_prefix('(') {
		let (_, declaration) = scope.split_once(')')?;

		return Some(declaration.trim_start());
	}

	Some(rest)
}

/// Reads the name before optional generic bounds, which may contain their own parentheses.
fn identifier_before_parameters(declaration: &str) -> Option<String> {
	let declaration = declaration.strip_prefix("r#").unwrap_or(declaration);
	let (head, _) = declaration.split_once('(')?;
	let identifier = head
		.split(|character: char| {
			!character.is_alphanumeric() && character != '_' && character != '$'
		})
		.next()?;

	super::last_identifier(identifier)
}
