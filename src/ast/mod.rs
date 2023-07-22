use pest::Parser;
use crate::first_child;
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
    let mut raw_tokens = MarkdownParser::parse(Rule::document, input)?;
    let document = Node::try_from(first_child!(raw_tokens)?)?;
    Ok(document)
}

pub fn parse_rule(input: &str, rule: Rule) -> Result<Node<'_>, ParseError> {
    let mut raw_tokens = MarkdownParser::parse(rule, input)?;
    let root = Node::try_from(first_child!(raw_tokens)?)?;
    Ok(root)
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
    pub fn markup_test() {
        let input = read_file_to_string("markup.md");
        let mut root = parse_rule(&input, Rule::paragraph)
            .unwrap_or_else(|e| panic!("Failed to parse document: {e}"));


        let mut markup_nodes = root.children_mut()
            .unwrap_or_else(|| panic!("Expected child nodes, but found none!"))
            .iter_mut();

        match markup_nodes.next() {
            Some(Node::Strong(strong)) => {
                let text_node = strong.children_mut()
                    .pop()
                    .unwrap_or_else(|| panic!("Missing text child for strong node"));
                assert_eq!(text_node.as_span(), "This text is strong");
            }
            node => panic!("Expected a strong text node, but found {node:?}"),
        };

        match markup_nodes.next() { 
            Some(Node::Emphasis(emphasis)) => {
                let text_node = emphasis.children_mut()
                    .pop()
                    .unwrap_or_else(|| panic!("Missing text child for emphasis node"));
                assert_eq!(text_node.as_span(), "This text is emphasized");
            }
            node => panic!("Expected an emphasis text node, but found {node:?}"),
        };

        match markup_nodes.next() { 
            Some(Node::Link(link)) => {
                assert_eq!(link.source(), "https://github.com");
                let text_node = link.children_mut()
                    .pop()
                    .unwrap_or_else(|| panic!("Missing text child for link node"));
                assert_eq!(text_node.as_span(), "this is a link");
            }
            node => panic!("Expected a link node, but found {node:?}"),
        };

        match markup_nodes.next() {
            Some(Node::Link(link)) => {
                assert_eq!(link.source(), "https://crates.io");
                let text_node = link.children_mut()
                    .pop()
                    .unwrap_or_else(|| panic!("Missing text child for link node"));
                assert_eq!(text_node.as_span(), "https://crates.io");
            }
            node => panic!("Expected a link node, but found {node:?}"),
        };

        match markup_nodes.next() {
            Some(Node::Image(img)) => {
                assert_eq!(img.source(), "https://tenor.com/oDMG.gif");
                assert_eq!(img.as_ref(), "huge mistake");
            }
            node => panic!("Expected a image node, but found {node:?}"),
        };

        match markup_nodes.next() {
            Some(Node::Code(code)) => {
                match code.children_mut().pop() {
                    Some(Node::Text(text)) => assert_eq!(text.as_ref(), r#"print("hello world!")"#),
                    node => panic!("Expected a text node, but found {node:?}"),
                }
            }
            node => panic!("Expected a code node, but found {node:?}"),
        }

        match markup_nodes.next() {
            Some(Node::Emphasis(emphasis)) => {
                let mut children = emphasis.children_mut().iter_mut();
                
                match children.next() {
                    Some(Node::Text(text)) => assert_eq!(text.as_ref(), "some "),
                    node => panic!("Expected a text node, but found {node:?}"),
                };
                
                match children.next() {
                    Some(Node::Strong(strong)) => {
                        match strong.children_mut().iter_mut().next() {
                            Some(Node::Text(text)) => assert_eq!(text.as_ref(), "bold and emphasized"),
                            node => panic!("Expected a text node, but found {node:?}"),
                        }
                    }
                    node => panic!("Expected a strong node, but found {node:?}"),
                }

                match children.next() {
                    Some(Node::Text(text)) => assert_eq!(text.as_ref(), " text"),
                    node => panic!("Expected a text node, but found {node:?}"),
                }
            } 
            node => panic!("Expected an emphasis node, but found {node:?}"),
        }

        match markup_nodes.next() {
            Some(Node::Strong(strong)) => {                
                match strong.children_mut().pop().as_mut() {
                    Some(Node::Link(link)) => {
                        assert_eq!(link.source(), "https://en.wikipedia.org/wiki/Where_no_man_has_gone_before");
                        match link.children_mut().pop() {
                            Some(Node::Text(text)) => assert_eq!(text.as_ref(), "to boldly go"),
                            node => panic!("Expected a text node, but found {node:?}"),
                        }
                    }
                    node => panic!("Expected a link node, but found {node:?}"),
                };
            } 
            node => panic!("Expected an strong node, but found {node:?}"),
        }
        
        match markup_nodes.next() {
            Some(Node::Emphasis(emphasis)) => {                
                match emphasis.children_mut().pop().as_mut() {
                    Some(Node::Code(code)) => {
                        match code.children_mut().pop() {
                            Some(Node::Text(text)) => assert_eq!(text.as_ref(), "echo 'hello world'"),
                            node => panic!("Expected a text node, but found {node:?}"),
                        }
                    }
                    node => panic!("Expected a code node, but found {node:?}"),
                };
            } 
            node => panic!("Expected an emphasis node, but found {node:?}"),
        }
    }

    #[test]
    pub fn list_test() {

    }

    #[test]
    pub fn blocks_test() {

    }
}
