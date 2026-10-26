---
"monostyle_languages": minor
"monostyle_lexer": patch
---

# A Ruby append is no longer read as a heredoc

Ruby's `<<` is also its append operator, and spacing is what tells the two apart: a heredoc's delimiter follows the marker immediately, so `list << item` appends and `class << self` opens a singleton class. Reading those as heredoc openers swallowed every line after them as string content, which mis-scored any Ruby file that appends to an array.

The heredoc syntax now says whether whitespace may separate the marker from the delimiter. Shell and PHP allow it (`cat << EOF`), so their behavior is unchanged; Ruby does not.
