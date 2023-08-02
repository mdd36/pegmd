use crate::ast::model::{CodeBlock, Heading, Link, List, Node, Reference, Image};
use crate::ast::traversal::{Direction, NextAction, Visitor};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;

#[derive(Default, Debug)]
pub struct LinkResolver<'a> {
    name_to_reference_table: RefCell<HashMap<&'a str, &'a Reference<'a>>>
}

impl <'a> LinkResolver<'a> {

    pub fn resolve(&self, name: &str) -> Option<&'a Reference<'a>> {
        self.name_to_reference_table.borrow()
            .get(name)
            .map(|reference| *reference)
    }

}

impl <'a> Visitor<'a> for LinkResolver<'a> {
    fn visit(&self, node: &'a Node<'a>, _direction: Direction) -> NextAction {
        match node {
            Node::Reference(reference) => {
                self.name_to_reference_table.borrow_mut().insert(reference.name(), reference);
                NextAction::GotoNext
            }
            Node::Document(_) => NextAction::GotoNext,
            _ => NextAction::SkipChildren 
        }
    }
}


#[derive(Debug)]
struct ListContext {
    tight: bool,
    _start: u32,
}

impl<'a> From<&List<'a>> for ListContext {
    fn from(value: &List<'a>) -> Self {
        Self {
            tight: value.tight(),
            _start: value.start(),
        }
    }
}

#[derive(Debug, Default)]
struct GenerationContext {
    list_context: Vec<ListContext>
}

impl GenerationContext {
    pub fn push_list_context(&mut self, context: &List) -> &Self {
        self.list_context.push(context.into());
        self
    }

    pub fn list_context(&self) -> Option<&ListContext> {
        self.list_context.last()
    }

    pub fn drop_list_context(&mut self) -> &Self {
        self.list_context.pop();
        self
    }
}

pub enum RenderError {
    IOError(String),
    StateError(String),
}

impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value.to_string())
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(e) => write!(f, "IOError ({e})"),
            Self::StateError(e) => write!(f, "StateError ({e})"),
        }
    }
}

/// An implementation of [`Visitor`] that generates HTML from AST.
#[derive(Default)]
pub struct HTMLRenderer<'a> {
    output: RefCell<Vec<u8>>,
    context: RefCell<GenerationContext>,
    link_table: LinkResolver<'a>,
}

/// A slightly nicer debug implementation that converts the output to a string rather than
/// writing the raw hex bytes.
impl <'a> std::fmt::Debug for HTMLRenderer<'a> {
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

impl <'a> HTMLRenderer<'a> {
    pub fn with_resolver(resolver: LinkResolver<'a>) -> Self {
        Self {
            link_table: resolver,
            ..Default::default()
        }
    }

    /// Create an HTML renderer with a pre-allocated buffer, ensuring that it will
    /// be able to hold at least `capacity` bytes without reallocating.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: RefCell::new(Vec::with_capacity(capacity)),
            ..Default::default()
        }
    }

    fn tag_with_attrs(
        &self,
        tag: &str,
        attrs: &[(&str, &str)],
        close: bool,
    ) -> Result<(), RenderError> {
        write!(self.output.borrow_mut(), "<{tag}")?;

        for (name, value) in attrs {
            write!(self.output.borrow_mut(), r#" {name}="{value}""#)?;
        }
        if close {
            write!(self.output.borrow_mut(), "/>")?;
        } else {
            write!(self.output.borrow_mut(), ">")?;
        }
        Ok(())
    }

    fn linebreak(&self) -> Result<(), RenderError> {
        write!(self.output.borrow_mut(), "</br>")?;
        Ok(())
    }

    fn link(&self, link: &Link, action: Direction) -> Result<(), RenderError> {
        if let Direction::Entering = action {
            let (source, title) = match self.link_table.resolve(link.source()) {
                Some(reference) => (reference.source(), reference.title()),
                None => (link.source(), link.title())
            };
            match title {
                Some(t) => self.tag_with_attrs("a", &[("href", source), ("title", t)], false)?,
                None => self.tag_with_attrs("a", &[("href", source)], false)?,
            };
        } else {
            write!(self.output.borrow_mut(), "</a>")?;
        }
        Ok(())
    }

    fn image(&self, image: &Image) -> Result<(), RenderError> {
        let source = image.source();
        let alt = image.as_span();
        self.tag_with_attrs("img", &[("src", source), ("alt", alt)], true)
    }

    fn document(&self, action: Direction) -> Result<(), RenderError> {
        if let Direction::Entering = action {
            write!(self.output.borrow_mut(), "<!DOCTYPE html><html>")?;
        } else {
            write!(self.output.borrow_mut(), "</html>")?;
        }
        Ok(())
    }

    fn paragraph(&self, action: Direction) -> Result<(), RenderError> {
        if let Direction::Entering = action {
            write!(self.output.borrow_mut(), "<p>")?;
        } else {
            write!(self.output.borrow_mut(), "</p>")?;
        }
        Ok(())
    }

    fn heading(&self, heading: &Heading, action: Direction) -> Result<(), RenderError> {
        if let Direction::Entering = action {
            write!(self.output.borrow_mut(), "<h{}>", heading.level())?;
        } else {
            write!(self.output.borrow_mut(), "</h{}>", heading.level())?;
        }
        Ok(())
    }

    fn list(&self, list: &List, action: Direction) -> Result<(), RenderError> {
        let start = list.start();

        if let Direction::Entering = action {
            self.context.borrow_mut().push_list_context(list);
            if list.ordered() {
                self.tag_with_attrs("ol", &[("start", &start.to_string())], false)?;
            } else {
                self.tag_with_attrs("ul", &[], false)?;
            }
        } else {
            self.context.borrow_mut().drop_list_context();
            if list.ordered() {
                write!(self.output.borrow_mut(), "</ol>")?;
            } else {
                write!(self.output.borrow_mut(), "</ul>")?;
            }
        }
        Ok(())
    }

    fn list_item(&self, action: Direction) -> Result<(), RenderError> {
        let context = self.context.borrow();
        let list_context = context.list_context().ok_or(RenderError::StateError(
            "No list context found when creating a list item".to_owned(),
        ))?;
        if let Direction::Entering = action {
            if list_context.tight {
                write!(self.output.borrow_mut(), "<li>")?;
            } else {
                write!(self.output.borrow_mut(), "<li><p>")?;
            }
        } else {
            if list_context.tight {
                write!(self.output.borrow_mut(), "</li>")?;
            } else {
                write!(self.output.borrow_mut(), "</p></li>")?;
            }
        }

        Ok(())
    }

    fn blockquote(&self, action: Direction) -> Result<(), RenderError> {
        if let Direction::Entering = action {
            write!(self.output.borrow_mut(), "<blockquote>")?;
        } else {
            write!(self.output.borrow_mut(), "</blockquote>")?;
        }
        Ok(())
    }

    fn codeblock(&self, codeblock: &CodeBlock, action: Direction) -> Result<(), RenderError> {
        if let Direction::Entering = action {
            write!(self.output.borrow_mut(), "<pre>")?;
            if let Some(language) = codeblock.language() {
                self.tag_with_attrs("code", &[("class", &format!("language-{language}"))], false)?;
            } else {
                write!(self.output.borrow_mut(), "<code>")?;
            }
        } else {
            write!(self.output.borrow_mut(), "")?;
            write!(self.output.borrow_mut(), "</code></pre>")?;
        }

        Ok(())
    }

    fn inline_style(&self, open: &str, close: &str, action: Direction) -> Result<(), RenderError> {
        match action {
            Direction::Entering => write!(self.output.borrow_mut(), "{}", open)?,
            Direction::Exiting => write!(self.output.borrow_mut(), "{}", close)?,
        };

        Ok(())
    }
}

impl <'a> Visitor<'_> for HTMLRenderer<'a> {
    fn visit(&self, node: &Node, action: Direction) -> NextAction {
        let emit_result = match node {
            Node::Document(_) => self.document(action),
            Node::Paragraph(_) => self.paragraph(action),
            Node::BlockQuote(_) => self.blockquote(action),
            Node::Heading(heading) => self.heading(heading, action),
            Node::List(list) => self.list(list, action),
            Node::ListItem(_) => self.list_item(action),
            Node::CodeBlock(cb) => self.codeblock(cb, action),
            Node::Emphasis(_) => self.inline_style("<em>", "</em>", action),
            Node::Strong(_) => self.inline_style("<strong>", "</strong>", action),
            Node::Code(_) => self.inline_style("<pre><code>", "</code></pre>", action),
            Node::Link(link) => self.link(link, action),
            Node::Image(img) => self.image(img),
            Node::Text(text) => {
                write!(self.output.borrow_mut(), "{}", text.as_span()).map_err(RenderError::from)
            }
            Node::Linebreak(_) => self.linebreak(),
            Node::SoftLinebreak(_) => {
                write!(self.output.borrow_mut(), " ").map_err(RenderError::from)
            }
            Node::Label(_) => return NextAction::GotoNext,
            Node::ThematicBreak(_) =>  {
                write!(self.output.borrow_mut(), "<hr/>").map_err(RenderError::from)
            }
            Node::Reference(_) => return NextAction::GotoNext,
            Node::EOI => Ok(()),
        };

        match emit_result {
            Ok(_) => NextAction::GotoNext,
            Err(e) => {
                println!("Encountered an error while generating HTML, stopping. Error was: {e}");
                NextAction::End
            }
        }
    }
}

impl <'a> Display for HTMLRenderer<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(self.output.borrow().as_slice()) {
            Ok(s) => write!(f, "{}", s),
            Err(e) => write!(f, "Invalid UTF-8 contents in buffer: {e:?}"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::parse_document;
    use crate::test_utils::read_file_to_string;
    use pretty_assertions::assert_eq;

    #[test]
    pub fn markup_test() {
        let input = read_file_to_string("markdown/markup.md");
        let root =
            parse_document(&input).unwrap_or_else(|e| panic!("Failed to parse document: {e}"));
        let link_resolver = LinkResolver::default();
        root.traverse(&link_resolver);
        let html_renderer = HTMLRenderer::with_resolver(link_resolver);
        root.traverse(&html_renderer);
        let actual = html_renderer.to_string();
        let expected = read_file_to_string("html/markup.html");
        assert_eq!(&actual, &expected);
    }

    #[test]
    pub fn list_test() {
        let input = read_file_to_string("markdown/lists.md");
        let root =
            parse_document(&input).unwrap_or_else(|e| panic!("Failed to parse document: {e}"));
        let html_renderer = HTMLRenderer::default();
        root.traverse(&html_renderer);
        let actual = html_renderer.to_string();
        let expected = read_file_to_string("html/lists.html");
        assert_eq!(&actual, &expected);
    }

    #[test]
    pub fn blocks_test() {
        let input = read_file_to_string("markdown/blocks.md");
        let root =
            parse_document(&input).unwrap_or_else(|e| panic!("Failed to parse document: {e}"));
        let html_renderer = HTMLRenderer::default();
        root.traverse(&html_renderer);
        let actual = html_renderer.to_string();
        let expected = read_file_to_string("html/blocks.html");
        assert_eq!(&actual, &expected);
    }
}
