use crate::error::ParseError;
use crate::first_child;
use crate::parser::{MarkdownParser, Rule};
use pest::Parser;

use self::model::Node;

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
    let mut raw_tokens = MarkdownParser::parse(Rule::document, input)?;
    let document = Node::try_from(first_child!(raw_tokens)?)?;
    Ok(document)
}

#[cfg(all(feature = "serde_support", test))]
pub mod test {
    use super::*;
    use crate::test_utils::read_file_to_string;
    use pretty_assertions::assert_eq;

    #[test]
    pub fn markup_test() {
        let input = read_file_to_string("markdown/markup.md");
        let root =
            parse_document(&input).unwrap_or_else(|e| panic!("Failed to parse document: {e}"));
        let actual = serde_json::to_string_pretty(&root)
            .unwrap_or_else(|e| panic!("Failed to serialize AST: {e}"));
        let expected = read_file_to_string("ast_json/markup.json");
        assert_eq!(&actual, &expected);
    }

    #[test]
    pub fn list_test() {
        let input = read_file_to_string("markdown/lists.md");
        let document =
            parse_document(&input).unwrap_or_else(|e| panic!("Failed to parse document: {e:?}"));
        let actual = serde_json::to_string_pretty(&document)
            .unwrap_or_else(|e| panic!("Failed to serialize AST: {e}"));
        let expected = read_file_to_string("ast_json/lists.json");
        assert_eq!(&actual, &expected);
    }

    #[test]
    pub fn blocks_test() {
        let input = read_file_to_string("markdown/blocks.md");
        let document =
            parse_document(&input).unwrap_or_else(|e| panic!("Failed to parse document: {e:?}"));
        let actual = serde_json::to_string_pretty(&document)
            .unwrap_or_else(|e| panic!("Failed to serialize AST: {e}"));
        let expected = read_file_to_string("ast_json/blocks.json");

        assert_eq!(&actual, &expected);
    }
}
