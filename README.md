# pegmd
Parses a non-standard Markdown flavor to an abstract syntax tree by defining its parsing expression grammar (PEG) with [pest](https://pest.rs/book/).

## Usage
The main function exported from the crate, `parse_document`, accepts a `&str` and on success returns a `Document` with the same lifetime as the input. From there, you can traverse the tree starting at the root using its `iter()` or `into_iter()` methods, which provide a stream of `Blocks`. Inline styles are defined by the `Text` enum and represent the leaf nodes of the tree.

## Unsupported
- Document streaming. Because `pest` lacks support for streaming, this crate also can't read a document from a stream.

## Markdown flavor notes
The parser's grammar deviates from [CommonMark v0.30](https://spec.commonmark.org/0.30/) in the following ways:

  - No support for HTML blocks.
  - No support for horizontal rules.
  - No support for Setext headings.
  - No support for link reference definitions.
  - No support for indented code blocks. You must use fenced code blocks instead.
  - Fenced code blocks can only be opened with a backtick (`), not a tilde (~)
  - Block quotes and verbatim elements are treated the same.
  - Surrounding text with underscores, \_like this\_, creates an underline style instead of italics. To italicize text, surround it with a single asterisk, \*like this\*.
  - Lists can't interrupt paragraphs, as is defined in `Markdown.pl`.

As an example, this file's syntax follows the rules implemented by the parser.
