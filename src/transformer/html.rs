use std::cell::RefCell;
use std::fmt::Display;
use std::io::{self, Write};
use crate::ast::model::{Node, Heading, Image, Link, List};
use crate::ast::traversal::{NextAction, Direction, Visitor};

#[derive(Debug, Default)]
struct GenerationContext {
  depth: RefCell<usize>,
  list_context: RefCell<Vec<bool>>,
}

impl  GenerationContext {

  pub fn increment_depth(&self) -> &Self {
    *self.depth.borrow_mut() += 1;
    self
  }

  pub fn decrement_depth(&self) -> &Self {
    *self.depth.borrow_mut() -= 1;
    self
  }

  pub fn depth(&self) -> usize {
    *self.depth.borrow()
  }

  pub fn push_list_context(&self, tight: bool) -> &Self {
    self.list_context.borrow_mut().push(tight);
    self
  }

  pub fn list_context(&self) -> Option<bool> {
    self.list_context.borrow().last().map(|context| *context)
  }

  pub fn drop_list_context(&self) -> &Self {
    self.list_context.borrow_mut().pop();
    self
  }

}

/// An implementation of [`Visitor`] that generates HTML from AST.
#[derive(Default)]
pub struct HTMLRenderer {
  output: RefCell<Vec<u8>>,
  context: GenerationContext,
}

/// A slightly nicer debug implementation that converts the output to a string rather than
/// writing the raw hex bytes.
impl std::fmt::Debug for HTMLRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(s) = std::str::from_utf8(self.output.borrow().as_slice()) {
          f.debug_struct("HTMLRenderer")
          .field("output", &s)
          .field("context", &self.context)
          .finish()
        } else {
          f.debug_struct("HTMLRenderer")
          .field("output", &self.output)
          .field("context", &self.context)
          .finish()
        }
    }
}

impl HTMLRenderer {

  /// Create an HTML renderer with a pre-allocated buffer, ensuring that it will 
  /// be able to hold at least `capacity` bytes without reallocating. 
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      output: RefCell::new(Vec::with_capacity(capacity)),
      ..Default::default()
    }
  }

  fn indent(&self) -> io::Result<()> {
    if self.context.depth() > 0 {
      write!(self.output.borrow_mut(), "{}", "  ".repeat(self.context.depth()))
    } else {
      Ok(())
    }
  }

  fn linebreak(&self) -> io::Result<()> {
    write!(self.output.borrow_mut(), "</br>")
  }

  fn link(&self, link: &Link, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      let source = link.source();
      write!(self.output.borrow_mut(), r#"<a href="{source}">"#)
    } else {
      write!(self.output.borrow_mut(), "</a>")
    }
  }

  fn image(&self, link: &Image) -> io::Result<()> {
      let source = link.source();
      let alt = link.as_ref();
      write!(self.output.borrow_mut(), r#"<img src="{source}" alt="{alt}">"#)
    
  }

  fn document(&self, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      self.context.increment_depth();
      write!(self.output.borrow_mut(), "<!DOCTYPE html>\n<html>")
    } else {
      self.context.decrement_depth();
      write!(self.output.borrow_mut(), "</html>")
    }
  }

  fn paragraph(&self, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      self.indent()?;
      write!(self.output.borrow_mut(), "\n<p>\n")?;
      self.context.increment_depth();
      self.indent()
    } else {
      self.context.decrement_depth();
      write!(self.output.borrow_mut(), "\n")?;
      self.indent()?;
      write!(self.output.borrow_mut(), "</p>")
    }
  }

  fn heading(&self, heading: &Heading, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      self.indent()?;
      write!(self.output.borrow_mut(), "<h{}>", heading.level())
    } else {
      write!(self.output.borrow_mut(), "</h{}>\n", heading.level())
    }
    
  }

  fn list(&self, list: &List, action: Direction) -> io::Result<()> {
    let tag = if list.ordered() { "ol"} else { "ul" };

    if let Direction::Entering = action {
      write!(self.output.borrow_mut(), "\n")?;
      self.indent()?;
      self.context
        .increment_depth()
        .push_list_context(list.tight());
      write!(self.output.borrow_mut(), "<{tag}>\n")
    } else {
      self.context
        .decrement_depth()
        .drop_list_context();
      self.indent()?;
      write!(self.output.borrow_mut(), "</{tag}>\n")
    }
    
  }

  fn list_item(&self, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      self.indent()?;
      if let Some(true) = self.context.list_context() {
        write!(self.output.borrow_mut(), "<li>")
      } else {
        write!(self.output.borrow_mut(), "<li><p>")
      }
    } else {
      if let Some(true) = self.context.list_context() {
        write!(self.output.borrow_mut(), "</li>\n")
      } else {
        write!(self.output.borrow_mut(), "</p></li>\n")
      }
    }
  }

  fn blockquote(&self, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      self.indent()?;
      write!(self.output.borrow_mut(), "<blockquote>")?;
      self.context.increment_depth();
      self.indent()
    } else {
      self.context.decrement_depth();
      write!(self.output.borrow_mut(), "\n")?;
      self.indent()?;
      write!(self.output.borrow_mut(), "</codeblock>\n")
    }
  }

  fn codeblock(&self, action: Direction) -> io::Result<()> {
    if let Direction::Entering = action {
      self.indent()?;
      write!(self.output.borrow_mut(), "<pre><code>")?;
      self.context.increment_depth();
      self.indent()
    } else {
      self.context.decrement_depth();
      write!(self.output.borrow_mut(), "\n")?;
      self.indent()?;
      write!(self.output.borrow_mut(), "</codeblock></pre>\n")
    }
  }

  fn inline_style(&self, open: &str, close: &str, action: Direction) -> io::Result<()> {
    match action {
      Direction::Entering => write!(self.output.borrow_mut(), "{}", open),
      Direction::Exiting => write!(self.output.borrow_mut(), "{}", close),
    }
  }
}

impl Visitor for HTMLRenderer {
    fn visit(&self, node: &Node, action: Direction) -> NextAction {
        let emit_result = match node {
          Node::Document(_) => self.document(action),
          Node::Paragraph(_) => self.paragraph(action),
          Node::BlockQuote(_) => self.blockquote(action),
          Node::Heading(heading) => self.heading(heading, action),
          Node::List(list) => self.list(list, action),
          Node::ListItem(_) => self.list_item(action),
          Node::CodeBlock(_) => self.codeblock(action),
          Node::Emphasis(_) => self.inline_style("<emp>", "</emp>", action),
          Node::Strong(_) => self.inline_style("<strong>", "</strong>", action),
          Node::Code(_) => self.inline_style("<pre><code>", "</code></pre>", action),
          Node::Link(link) => self.link(link, action),
          Node::Image(img) => self.image(img),
          Node::Text(text) => write!(self.output.borrow_mut(), "{}", text.as_ref()),
          Node::Linebreak(_) => self.linebreak(),
          Node::Label(_) => return NextAction::GotoNext,
          Node::EOI => Ok(())
        };

       match emit_result {
        Ok(_) => NextAction::GotoNext,
        Err(e) => {
          println!("Encountered an error while generating HTML, stopping. Error was: {e:?}");
          NextAction::End
        }
       }
    }
}

impl Display for HTMLRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(self.output.borrow().as_slice()) {
          Ok(s) => write!(f, "{}", s),
          Err(e) => write!(f, "Invalid UTF-8 contents in buffer: {e:?}"),
        }
    }
}

#[cfg(test)]
mod test {
    use pest::Parser;

    use crate::{ast::parse_document, parser::{MarkdownParser, Rule}};

    use super::*;
    
    #[test]
    pub fn basic_test() {
        let input = "
*italic*

- *italics*";
        let document = parse_document(input)
          .unwrap_or_else(|e| panic!("Error while parsing the document: {e}"));
        println!("{:#?}", MarkdownParser::parse(Rule::document, input));
        println!("--------------");
        // println!("{document:#?}");
        // println!("---------------------");
        let html_renderer = HTMLRenderer::default();
        document.traverse(&html_renderer);
        println!("{}", html_renderer.to_string());
    }

    #[test]
    pub fn html_complex_test() {
        let contents = std::fs::read_to_string("src/test.md")
            .unwrap_or_else(|e| panic!("Failed to open file: {e:?}"));

        let document = parse_document(&contents);
        println!("{document:#?}");
        println!("---------------------");
        let html_renderer = HTMLRenderer::default();
        document.unwrap().traverse(&html_renderer);
        println!("{}", html_renderer.to_string());
    }
}