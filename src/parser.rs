use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "markdown.pest"]
pub struct MarkdownParser;

impl Rule {

  pub fn is_plaintext(&self) -> bool {
    match self {
      Self::str | Self::symbol | Self::escaped_special_char | 
      Self::source | Self::space | Self::non_space => true,
      _ => false
    }
  }

}