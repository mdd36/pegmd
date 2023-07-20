use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::fmt::Display;
use std::io::{Write, Result};
use crate::v2::{WalkAction, Visitor, Link, Heading, Node, Image, NextAction, List};

type WriteResult = Result<()>;

#[derive(Debug, Default)]
struct GenerationContext {
  depth: RefCell<usize>,
  is_list_tight: RefCell<bool>,
}

impl GenerationContext {

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

  pub fn set_is_list_tight(&self, is_ordered: bool) -> &Self {
    *self.is_list_tight.borrow_mut() = is_ordered;
    self
  }

  pub fn is_list_tight(&self) -> bool {
    *self.is_list_tight.borrow()
  }

}

#[derive(Default, Debug)]
struct HTMLRenderer {
  output: RefCell<Vec<u8>>,
  context: GenerationContext,
}

impl HTMLRenderer {

  fn indent(&self) -> WriteResult {
    if self.context.depth() > 0 {
      write!(self.output.borrow_mut(), "{}", "  ".repeat(self.context.depth()))
    } else {
      Ok(())
    }
  }

  fn linebreak(&self) -> WriteResult {
    write!(self.output.borrow_mut(), "</br>\n")
  }

  fn link(&self, link: &Link, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
      let source = link.source();
      write!(self.output.borrow_mut(), r#"<a href="{source}">"#)
    } else {
      write!(self.output.borrow_mut(), "</a>")
    }
  }

  fn image(&self, link: &Image) -> WriteResult {
      let source = link.source();
      let alt = link.as_ref();
      write!(self.output.borrow_mut(), r#"<img src="{source}" alt="{alt}">"#)
    
  }

  fn document(&self, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
      self.context.increment_depth();
      write!(self.output.borrow_mut(), "<!DOCTYPE html>\n<html>\n")
    } else {
      self.context.decrement_depth();
      write!(self.output.borrow_mut(), "</html>")
    }
  }

  fn paragraph(&self, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
      self.indent()?;
      write!(self.output.borrow_mut(), "<p>\n")?;
      self.context.increment_depth();
      self.indent()
    } else {
      self.context.decrement_depth();
      write!(self.output.borrow_mut(), "\n")?;
      self.indent()?;
      write!(self.output.borrow_mut(), "</p>\n")
    }
  }

  fn heading(&self, heading: &Heading, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
      self.indent()?;
      write!(self.output.borrow_mut(), "<h{}>", heading.level())
    } else {
      write!(self.output.borrow_mut(), "</h{}>\n", heading.level())
    }
    
  }

  fn list(&self, list: &List, action: WalkAction) -> WriteResult {
    let tag = if list.ordered() { "ol"} else { "ul" };
    self.context.set_is_list_tight(list.tight());

    if let WalkAction::Enter = action {
      self.indent()?;
      self.context.increment_depth();
      write!(self.output.borrow_mut(), "<{tag}>\n")
    } else {
      self.context.decrement_depth();
      self.indent()?;
      write!(self.output.borrow_mut(), "</{tag}>\n")
    }
    
  }

  fn list_item(&self, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
      self.indent()?;
      if self.context.is_list_tight() {
        write!(self.output.borrow_mut(), "<li>")
      } else {
        write!(self.output.borrow_mut(), "<li><p>")
      }
    } else {
      if self.context.is_list_tight() {
        write!(self.output.borrow_mut(), "</li>\n")
      } else {
        write!(self.output.borrow_mut(), "</p></li>\n")
      }
    }
  }

  fn blockquote(&self, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
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

  fn codeblock(&self, action: WalkAction) -> WriteResult {
    if let WalkAction::Enter = action {
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

  fn inline_style(&self, open: &str, close: &str, action: WalkAction) -> WriteResult {
    match action {
      WalkAction::Enter => write!(self.output.borrow_mut(), "{}", open),
      WalkAction::Leave => write!(self.output.borrow_mut(), "{}", close),
    }
  }
}

impl Visitor for HTMLRenderer {
    fn visit(&self, node: &crate::v2::Node, action: WalkAction) -> crate::v2::NextAction {
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
          Node::Link(link) => self.link(link, action),
          Node::Image(img) => self.image(img),
          Node::Text(text) => write!(self.output.borrow_mut(), "{}", text.as_ref()),
          Node::Linebreak(_) => write!(self.output.borrow_mut(), "</br>"),
          Node::Code(text) => write!(self.output.borrow_mut(), "<pre><code>{}</code></pre>", text.as_ref()),
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

    use crate::{v2::{parse_document, walk}, parser::{MarkdownParser, Rule}};

    use super::*;
    
    #[test]
    pub fn basic_test() {
        let input = "
- Item one
  - A sublist

  - That's loose
- Item two";
        let document = parse_document(input);
        println!("{:#?}", MarkdownParser::parse(Rule::document, input));
        println!("--------------");
        println!("{document:?}");
        println!("---------------------");
        let html_renderer = HTMLRenderer::default();
        walk(&document.unwrap(), &html_renderer);
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
        walk(&document.unwrap(), &html_renderer);
        println!("{}", html_renderer.to_string());
    }
}

// #[derive(Default)]
// struct HTMLConverter<'a> {
//     text_buffer: RefCell<Vec<Leaf<'a>>>,
//     document: RefCell<Vec<String>>,
//     container_stack: RefCell<Vec<(&'static str, &'static str)>>,
// }

// impl Display for HTMLConverter {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "<!DOCTYPE html>\n<html>")?;
//         self.document.borrow().iter()
//             .map(|segment| write!(f, "{}", segment))
//             .collect::<std::fmt::Result>()?;
//         write!(f, "</html>")
//     }
// }

// impl Visitor for HTMLConverter{
//     fn visit(&self, node: &Node, action: WalkAction) -> NextAction {
//         match node {
//             Node::Container(c) => self.emit_container(c, action),
//             Node::Leaf(l) => self.emit_leaf(l),
//             Node::EOI => return NextAction::GotoNext,
//         }
//     }
// }

// impl HTMLConverter {

//     fn emit_container(&self, container: &Container, action: WalkAction) -> NextAction {
//         if let WalkAction::Leave = action {
//             let (open, close) = self.container_stack.borrow_mut().pop().unwrap();
//             self.document.borrow_mut().push(format!("{open}{}{close}", self.text_buffer.borrow()));
//             self.text_buffer.borrow_mut().clear();
//             return NextAction::GotoNext;
//         }

//         match &container.container_type {
//             ContainerType::Document => self.container_stack.borrow_mut().push(("<html>", "</html>")),
//             ContainerType::Paragraph => self.container_stack.borrow_mut().push(("<p>", "</p>")),
//             ContainerType::BulletList => self.container_stack.borrow_mut().push(("<ul>", "</ul>")),
//             ContainerType::OrderedList => self.container_stack.borrow_mut().push(("<ol>", "</ol>")),
//             ContainerType::Emphasis => self.container_stack.borrow_mut().push(("<emp>", "</emp>")),
//             ContainerType::Strong => self.container_stack.borrow_mut().push(("<strong>", "</strong>")),
//             ContainerType::ListItem => self.container_stack.borrow_mut().push(("<li>", "</li>")),
//             ContainerType::BlockQuote => self.container_stack.borrow_mut().push(("<blockquote>", "</blockquote>")),
//             ContainerType::Link | ContainerType::Image | ContainerType::Title => {},
//             _ => todo!()
//         }

//         NextAction::GotoNext
//     }

//     fn emit_leaf(&self, leaf: &Leaf) -> NextAction {
//         match &leaf.leaf_type {
//             LeafType::Text => self.text_buffer.borrow_mut().push_str(leaf),
//             LeafType::Linebreak => self.document.borrow_mut().push("<br/>".to_owned()),
//             LeafType::Destination => self.
//             _ => todo!()
//         };

//         NextAction::GotoNext
//     }
// }