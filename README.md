# pegmd
Parses a non-standard Markdown flavor to an abstract syntax tree by defining its parsing expression grammar (PEG) with [pest](https://pest.rs/book/).

## Features
  - Parse from any source that implements `Into<str>`.
  - (WIP) Transform the AST to another format using one of the built-in transformers. Requires the `transformers` feature.

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
  - A blank line is required between a paragraph and start a new format type, like headers, lists, quotes, or codeblocks.

As an example, this file's syntax follows the rules implemented by the parser.

## Usage
TODO -- will complete when the final interface is defined.