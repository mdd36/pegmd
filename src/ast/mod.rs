use pest::Parser;
use crate::parser::{MarkdownParser, Rule};
use crate::error::ParseError;

use self::model::{Node, Document};

mod macros;

pub mod model;
pub mod traversal;

/// Generate an abstract syntax tree (AST) for the markdown document. Since the AST nodes
/// store segments of the slice in their leaves, the lifetime of the AST is tied to that of
/// the input.
/// 
/// ### Parameters
/// - `input` - The markdown source.
/// 
///  
/// ### Returns
/// A result that on success contains the root of the AST, and on failure a [`ParseError`].
pub fn parse_document(input: &str) -> Result<Node<'_>, ParseError> { 
    let raw_tokens = MarkdownParser::parse(Rule::document, input)?;
    let blocks: Result<Vec<Node>, ParseError> = raw_tokens.into_iter()
        .map(Node::try_from)
        .collect();
    Ok(Node::Document(Document::new(blocks?)))
}


#[cfg(test)]
mod test {
    use std::{fs::read_to_string, path::PathBuf};
    use super::*;

    fn read_file_to_string(file_name: &str) -> String {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_data/");
        path.push(file_name);

        read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read file {path:?} to string: {e}"))
    }
    
    #[test]
    pub fn basic_test() {
        let input = read_file_to_string("markup.md");
        let document = parse_document(&input)
            .unwrap_or_else(|e| panic!("Failed to parse document: {e}"));

        println!("{document:?}");

        
    }

    #[test]
    pub fn complex_test() {
        let contents = std::fs::read_to_string("src/test.md")
            .unwrap_or_else(|e| panic!("Failed to open file: {e:?}"));

        println!("{:#?}", MarkdownParser::parse(Rule::document, &contents));
        println!("--------------");
        let document = parse_document(&contents);

        println!("{document:#?}");
    }
}
