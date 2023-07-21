# pegmd
Parses a Markdown document that follows [CommonMark v0.30](https://spec.commonmark.org/0.30/) to an abstract syntax tree by defining its parsing expression
grammar (PEG) with [pest](https://pest.rs/book/). The crate also optionally provides a transformer to emit the AST as HTML if the `html` feature included.

## Usage

### Creating an AST
The main function exported from the crate, `ast::parse_document`, accepts a `&str` and on success returns a `Node` with the same lifetime as the input. 

### Traversal
From there, you can traverse the tree by creating a struct that implements the `traversal::Vistor` trait and providing it to the `Node::traverse` method.

### HTML Conversion
If the `html` feature is enabled, the crate provides the `html::HTMLTransformer` struct that implements the `Visitor` trait to create a well-formatted HTML output.

## Unsupported
- Document streaming. Because `pest` lacks support for streaming, this crate also can't read a document from a stream.

## Current Limitations
While the end goal for the parser is to support the entire CommonMark spec, it currently doesn't support:

- HTML blocks
- Horizontal rules
- Setext headings
- Entity references
- Indented code blocks. You must use fenced code blocks instead
- Fenced code blocks can only be opened with a backtick (`), not a tilde (~)
- Lists can't interrupt paragraphs

All of these are a WIP and will be added to the parser.
