//! Language identity.

use serde::Deserialize;
use serde::Serialize;

/// A programming language monostyle can analyze.
///
/// The set starts from the languages supported by `mozilla/rust-code-analysis`
/// and adds Dart plus a set of widely used languages that project does not cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
	// Languages inherited from rust-code-analysis.
	/// C.
	C,
	/// C++.
	Cpp,
	/// C#.
	#[serde(rename = "csharp")]
	CSharp,
	/// Java.
	Java,
	/// JavaScript.
	#[serde(rename = "javascript")]
	JavaScript,
	/// Kotlin.
	Kotlin,
	/// `SpiderMonkey`'s JavaScript dialect.
	Mozjs,
	/// Python.
	Python,
	/// Rust.
	Rust,
	/// TypeScript.
	#[serde(rename = "typescript")]
	TypeScript,
	/// TypeScript with JSX.
	Tsx,

	// Dart, the language this project was built for.
	/// Dart.
	Dart,

	// Additional popular languages beyond rust-code-analysis.
	/// Go.
	Go,
	/// Swift.
	Swift,
	/// Ruby.
	Ruby,
	/// PHP.
	Php,
	/// Scala.
	Scala,
	/// Shell (POSIX sh, bash, zsh).
	Shell,
	/// Lua.
	Lua,
	/// Elixir.
	Elixir,
	/// Haskell.
	Haskell,
	/// Nix.
	Nix,

	// Non-source documents.
	/// Markdown, analyzed for the readability of its embedded code fences.
	Markdown,
}

impl Language {
	/// Every language, in report order.
	pub const ALL: [Self; 23] = [
		Self::C,
		Self::Cpp,
		Self::CSharp,
		Self::Java,
		Self::JavaScript,
		Self::Kotlin,
		Self::Mozjs,
		Self::Python,
		Self::Rust,
		Self::TypeScript,
		Self::Tsx,
		Self::Dart,
		Self::Go,
		Self::Swift,
		Self::Ruby,
		Self::Php,
		Self::Scala,
		Self::Shell,
		Self::Lua,
		Self::Elixir,
		Self::Haskell,
		Self::Nix,
		Self::Markdown,
	];

	/// The canonical display name.
	#[must_use]
	pub const fn name(self) -> &'static str {
		match self {
			Self::C => "c",
			Self::Cpp => "cpp",
			Self::CSharp => "csharp",
			Self::Java => "java",
			Self::JavaScript => "javascript",
			Self::Kotlin => "kotlin",
			Self::Mozjs => "mozjs",
			Self::Python => "python",
			Self::Rust => "rust",
			Self::TypeScript => "typescript",
			Self::Tsx => "tsx",
			Self::Dart => "dart",
			Self::Go => "go",
			Self::Swift => "swift",
			Self::Ruby => "ruby",
			Self::Php => "php",
			Self::Scala => "scala",
			Self::Shell => "shell",
			Self::Lua => "lua",
			Self::Elixir => "elixir",
			Self::Haskell => "haskell",
			Self::Nix => "nix",
			Self::Markdown => "markdown",
		}
	}

	/// File extensions that map to this language, without a leading dot.
	///
	/// The first entry is treated as canonical when a language needs a suffix for
	/// synthetic files such as Markdown code fences.
	#[must_use]
	pub const fn extensions(self) -> &'static [&'static str] {
		match self {
			Self::C => &["c", "h"],
			Self::Cpp => &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
			Self::CSharp => &["cs"],
			Self::Java => &["java"],
			Self::JavaScript => &["js", "mjs", "cjs"],
			Self::Kotlin => &["kt", "kts"],
			Self::Mozjs => &["mozjs"],
			Self::Python => &["py", "pyi"],
			Self::Rust => &["rs"],
			Self::TypeScript => &["ts", "mts", "cts"],
			Self::Tsx => &["tsx"],
			Self::Dart => &["dart"],
			Self::Go => &["go"],
			Self::Swift => &["swift"],
			Self::Ruby => &["rb"],
			Self::Php => &["php"],
			Self::Scala => &["scala", "sc"],
			Self::Shell => &["sh", "bash", "zsh"],
			Self::Lua => &["lua"],
			Self::Elixir => &["ex", "exs"],
			Self::Haskell => &["hs"],
			Self::Nix => &["nix"],
			Self::Markdown => &["md", "markdown"],
		}
	}

	/// Resolves a language from a file extension.
	#[must_use]
	pub fn from_extension(extension: &str) -> Option<Self> {
		let normalized = extension.trim_start_matches('.').to_ascii_lowercase();

		Self::ALL
			.into_iter()
			.find(|language| language.extensions().contains(&normalized.as_str()))
	}

	/// Resolves a language from a Markdown code-fence info string.
	///
	/// Fence info strings vary a lot in practice (`rs`, `rust`, `Rust`, `typescript`,
	/// `ts`, or a bare `shell`), so this accepts aliases in addition to the canonical
	/// name and extensions.
	#[must_use]
	pub fn from_fence_tag(tag: &str) -> Option<Self> {
		let normalized = tag.trim().to_ascii_lowercase();
		let primary = normalized.split_whitespace().next().unwrap_or_default();

		if primary.is_empty() {
			return None;
		}

		if let Some(language) = Self::from_name(primary) {
			return Some(language);
		}

		if let Some(language) = Self::from_extension(primary) {
			return Some(language);
		}

		let aliased = match primary {
			"c++" | "cplusplus" => Some(Self::Cpp),
			"c#" | "csharp" => Some(Self::CSharp),
			"dart" => Some(Self::Dart),
			"golang" => Some(Self::Go),
			"hs" => Some(Self::Haskell),
			"js" | "node" => Some(Self::JavaScript),
			"jsx" => Some(Self::Tsx),
			"kt" => Some(Self::Kotlin),
			"md" => Some(Self::Markdown),
			"py" => Some(Self::Python),
			"rb" => Some(Self::Ruby),
			"sh" | "shell-session" | "console" | "bash" | "zsh" => Some(Self::Shell),
			"ts" => Some(Self::TypeScript),
			_ => None,
		};

		aliased.or_else(|| {
			Self::ALL.iter().copied().find(|language| {
				let name = language.name();

				name.contains(primary) || primary.contains(name)
			})
		})
	}

	/// Resolves a language from its canonical name.
	#[must_use]
	pub fn from_name(name: &str) -> Option<Self> {
		let normalized = name.trim().to_ascii_lowercase();

		Self::ALL
			.into_iter()
			.find(|language| language.name() == normalized)
	}

	/// The file extension used for synthetic files, such as Markdown code fences.
	#[must_use]
	pub fn canonical_extension(self) -> &'static str {
		self.extensions().first().copied().unwrap_or("txt")
	}

	/// Whether this language is configured from the shared C-family profile.
	#[must_use]
	pub const fn is_c_family(self) -> bool {
		matches!(
			self,
			Self::C
				| Self::Cpp | Self::CSharp
				| Self::Java | Self::JavaScript
				| Self::Kotlin
				| Self::Mozjs
				| Self::Rust | Self::TypeScript
				| Self::Tsx | Self::Dart
				| Self::Go | Self::Swift
				| Self::Php | Self::Scala
		)
	}
}

impl std::fmt::Display for Language {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str(self.name())
	}
}
