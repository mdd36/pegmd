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

impl std::fmt::Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
          Self::TokenizationError(msg) => write!(f, "Failed to lex input string to tokens: {msg}"),
          Self::SyntaxError(msg) => write!(f, "Invalid structure found in document: {msg}"),
      }
  }
}