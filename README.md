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

- [Reference links](https://spec.commonmark.org/0.30/#link-reference-definitions)
- [HTML blocks](https://spec.commonmark.org/0.30/#html-blocks)
- [Setext headings](https://spec.commonmark.org/0.30/#setext-headings)
- [ATX headings with closing hashes](https://spec.commonmark.org/0.30/#example-71)
- [Entity references](https://spec.commonmark.org/0.30/#entity-and-numeric-character-references)
- [Using a tilde (~) to fence a codeblock](https://spec.commonmark.org/0.30/#example-120)
- Fenced codeblocks without a closing fence run until the end of the document rather than to the [end of the container block](https://spec.commonmark.org/0.30/#example-126)
- Some of the edge cases for block quotes aren't handled per the spec. Specifically examples [247](https://spec.commonmark.org/0.30/#example-247) through [252](https://spec.commonmark.org/0.30/#example-252) 

All of these are a WIP and will be added to the parser.
