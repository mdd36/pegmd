
extern crate alloc;
mod parser;

pub use parser::{parse_document, ParseError, Document, Section, Text};