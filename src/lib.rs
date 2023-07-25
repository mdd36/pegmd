pub mod ast;
pub mod transformer;

mod parser {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "markdown.pest"]
    pub struct MarkdownParser;

    impl Rule {
        pub fn is_plaintext(&self) -> bool {
            matches!(
                self,
                Self::str
                    | Self::symbol
                    | Self::escaped_special_char
                    | Self::source
                    | Self::space
                    | Self::non_space
            )
        }
    }
}

pub mod error {
    use crate::parser::Rule;

    #[derive(Debug)]
    pub enum ParseError {
        TokenizationError(String),
        SyntaxError(String),
    }

    impl From<pest::error::Error<Rule>> for ParseError {
        fn from(value: pest::error::Error<Rule>) -> Self {
            ParseError::TokenizationError(value.to_string())
        }
    }

    impl From<core::num::ParseIntError> for ParseError {
        fn from(value: core::num::ParseIntError) -> Self {
            ParseError::SyntaxError(value.to_string())
        }
    }

    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::TokenizationError(msg) => {
                    write!(f, "Failed to lex input string to tokens: {msg}")
                }
                Self::SyntaxError(msg) => write!(f, "Invalid structure found in document: {msg}"),
            }
        }
    }
}

#[cfg(test)]
mod test_utils {
    use std::{fs::read_to_string, path::PathBuf};

    pub fn read_file_to_string(file_name: &str) -> String {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_data/");
        path.push(file_name);

        read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read file {path:?} to string: {e}"))
    }
}
