# Supported languages

<!-- {=supportedLanguages} -->

Rust, C, C++, C#, Java, JavaScript, Kotlin, Mozjs, Python, TypeScript, TSX, **Dart**, Go, Swift,
Ruby, PHP, Scala, Shell, Lua, Elixir, Haskell, Nix, and Markdown.

The first eleven match what [`rust-code-analysis`](https://github.com/mozilla/rust-code-analysis)
supports, so numbers from the two tools are comparable. Dart is included because it is the language
this tool was built for. The remaining ten cover widely used languages that project does not reach.

<!-- {/supportedLanguages} -->

Languages are described by data profiles rather than parsers, so a language is added by editing
`crates/monostyle_languages/src/catalog.rs` rather than by writing analysis code.
